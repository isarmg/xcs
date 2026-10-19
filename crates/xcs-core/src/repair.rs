//! Repair the original unpublished candidate using complete verified bytes.
use crate::{disk::*, engine::Prepared, journal::Candidate, *};
use std::{
    fs,
    path::{Path, PathBuf},
};
use xcs_platform::{
    fs_safety::linux::{MountPolicy, OpenAt2Root},
    payload::{Expectation, verify_full},
};

fn required(message: impl Into<String>) -> Error {
    Error::new("RECOVERY_REQUIRED", message)
}
fn matching(path: &Path, version: &FileVersion) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(required(format!("cannot inspect recovery bytes: {e}"))),
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            return Err(required("recovery source is not a regular non-linked file"));
        }
        Ok(meta) if meta.len() != version.logical_size => return Ok(false),
        Ok(_) => {}
    }
    let mut file = open_regular(path)?;
    let hash = blake3::Hash::from_hex(&version.content_hash)
        .map_err(|_| required("candidate content digest is invalid"))?;
    match verify_full(
        &mut file,
        Expectation {
            bytes: version.logical_size,
            blake3: Some(hash),
        },
    ) {
        Ok(_) => Ok(true),
        Err(xcs_platform::payload::Error::Digest | xcs_platform::payload::Error::Length { .. }) => {
            Ok(false)
        }
        Err(e) => Err(required(format!("cannot verify recovery bytes: {e}"))),
    }
}
impl Engine {
    /// The same source discovery is used to reject a permanent-loss declaration
    /// while known complete bytes can still repair the frozen candidate.
    pub(crate) fn pending_payload_source(
        &self,
        version: &FileVersion,
        placement: &Placement,
    ) -> Result<Option<PathBuf>> {
        for slot in &placement.route_slot_ids {
            let disk = self.disk(slot)?;
            validate_seal(&self.config, disk)?;
            let directory = version_dir(disk, &version.history_tree_id, &version.version_id);
            let source = crate::container_storage::content_path(&directory, version)?;
            if matching(&source, version)? {
                return Ok(Some(source));
            }
        }
        for workspace in self.workspaces.values() {
            let directory = self.workspace_data_directory(workspace)?;
            if !directory.exists() {
                continue;
            }
            // Older generations remain valid recovery candidates even after
            // the current workspace manifest points at newer dirty bytes.
            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                if entry
                    .path()
                    .extension()
                    .is_some_and(|suffix| suffix == "full")
                    && matching(&entry.path(), version)?
                {
                    return Ok(Some(entry.path()));
                }
            }
        }
        for published in self.state.versions.values().filter(|v| {
            v.content_hash == version.content_hash && v.logical_size == version.logical_size
        }) {
            if self
                .state
                .content_states
                .get(&published.version_id)
                .is_some_and(|s| s != "AVAILABLE")
            {
                continue;
            }
            if let Ok((source, _)) = self.valid_replica(&published.version_id) {
                if matching(&source, version)? {
                    return Ok(Some(source));
                }
            }
        }
        for disk in &self.config.disks {
            validate_seal(&self.config, disk)?;
            let quarantine = disk.root.join("quarantine");
            if !quarantine.exists() {
                continue;
            }
            for entry in fs::read_dir(&quarantine)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir()
                    || !entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{}-repair-", version.version_id))
                {
                    continue;
                }
                // Quarantine is an inspection bundle, independently of the
                // live container layout used by the original version.
                let source = entry.path().join("content").join(&version.creation_name);
                if matching(&source, version)? {
                    return Ok(Some(source));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn repair_pending_payloads(&self, candidate: &Candidate) -> Result<()> {
        for evidence in &candidate.versions {
            let version = &evidence.version;
            let mut damaged = Vec::new();
            for slot in &evidence.placement.route_slot_ids {
                let disk = self.disk(slot)?;
                validate_seal(&self.config, disk)?;
                let directory = version_dir(disk, &version.history_tree_id, &version.version_id);
                let intact =
                    read_json::<FileVersion>(&directory.join("version.json")).is_ok_and(|stored| {
                        object_hash("version", &stored).ok() == object_hash("version", version).ok()
                    }) && verify_payload(&directory, version).is_ok();
                if !intact {
                    damaged.push((disk.clone(), directory));
                }
            }
            if damaged.is_empty() {
                continue;
            }
            let mut source = self
                .pending_payload_source(version, &evidence.placement)?
                .ok_or_else(|| {
                    required(format!(
                        "no verified complete bytes can repair {}; inspect permanent-loss recovery",
                        version.version_id
                    ))
                })?;
            for (disk, directory) in damaged {
                let quarantine = disk.root.join("quarantine");
                private_dir(&quarantine)?;
                let name = format!("{}-repair-{}", version.version_id, new_id("req"));
                let bundle = quarantine.join(&name);
                private_dir(&bundle)?;
                let content = crate::container_storage::content_path(&directory, version)?;
                match fs::symlink_metadata(&content) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                    Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                        return Err(required("damaged payload is not a trusted regular file"));
                    }
                    Ok(_) => {
                        let destination = bundle.join("content");
                        private_dir(&destination)?;
                        let from = OpenAt2Root::open(
                            content.parent().expect("content parent"),
                            MountPolicy::SameMount,
                        )
                        .map_err(|e| required(e.to_string()))?;
                        let to = OpenAt2Root::open(&destination, MountPolicy::SameMount)
                            .map_err(|e| required(e.to_string()))?;
                        rustix::fs::renameat_with(
                            from.directory(),
                            content.file_name().expect("payload name"),
                            to.directory(),
                            version.creation_name.as_str(),
                            rustix::fs::RenameFlags::NOREPLACE,
                        )
                        .map_err(std::io::Error::from)?;
                        from.directory().sync_all()?;
                        to.directory().sync_all()?;
                        if source == content {
                            source = destination.join(&version.creation_name);
                        }
                    }
                }
                match fs::symlink_metadata(&directory) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(e.into()),
                    Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                        return Err(required("damaged replica path is not a trusted directory"));
                    }
                    Ok(_) => {}
                }
                let parent = directory
                    .parent()
                    .ok_or_else(|| required("replica parent missing"))?;
                let from = OpenAt2Root::open(parent, MountPolicy::SameMount)
                    .map_err(|e| required(e.to_string()))?;
                let to = OpenAt2Root::open(&bundle, MountPolicy::SameMount)
                    .map_err(|e| required(e.to_string()))?;
                rustix::fs::renameat_with(
                    from.directory(),
                    directory.file_name().expect("version directory name"),
                    to.directory(),
                    "metadata",
                    rustix::fs::RenameFlags::NOREPLACE,
                )
                .map_err(std::io::Error::from)?;
                from.directory().sync_all()?;
                to.directory().sync_all()?;
            }
            self.prepare_payload(&Prepared {
                version: version.clone(),
                placement: evidence.placement.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn pending() -> (tempfile::TempDir, PathBuf, Config, String, String) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("state");
        let config = Config::development([
            temp.path().join("p"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&root, config.clone()).unwrap();
        let repo = engine
            .create_repository("repair", "P01", 3, "admin", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "a", b"verified original bytes")
            .unwrap();
        let operation = new_id("op");
        engine.inject_fault_once("after_accept_2");
        assert_eq!(
            engine
                .commit_workspace(CommitRequest {
                    workspace_id: workspace.workspace_id,
                    operation_id: operation.clone(),
                    principal: "admin".into(),
                    expected_head: workspace.base_head,
                    fencing_token: workspace.fencing_token,
                    message: String::new(),
                    request_digest: None
                })
                .unwrap_err()
                .code(),
            "IN_DOUBT"
        );
        drop(engine);
        (temp, root, config, repo.main_branch_id, operation)
    }
    #[test]
    fn recovery_repairs_missing_replicas_without_replacing_the_candidate_or_route() {
        let (_temp, root, config, branch, operation) = pending();
        let histories = config.disks[0].root.join("histories");
        let history = fs::read_dir(&histories)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let version = fs::read_dir(history.join("versions"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let metadata: FileVersion = read_json(&version.join("version.json")).unwrap();
        fs::remove_dir_all(&version).unwrap();
        let backup = version_dir(
            &config.disks[1],
            &metadata.history_tree_id,
            &metadata.version_id,
        );
        fs::remove_file(backup.join("checksums.bin")).unwrap();
        let engine = Engine::open(&root).unwrap();
        assert_eq!(
            engine.operation(&operation).unwrap().commit_state,
            "PUBLISHED"
        );
        let entries = engine.list_tree(&branch).unwrap();
        assert_eq!(entries[0].version_id, metadata.version_id);
        let mut opened = engine.open_content(&metadata.version_id).unwrap();
        let mut bytes = Vec::new();
        opened.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"verified original bytes");
        assert!(engine.check().unwrap().issues.is_empty());
        assert!(
            fs::read_dir(config.disks[1].root.join("quarantine"))
                .unwrap()
                .next()
                .is_some()
        );
    }
    #[test]
    fn recovery_uses_retained_workspace_bytes_when_all_candidate_replicas_are_lost() {
        let (_temp, root, config, branch, operation) = pending();
        for disk in &config.disks {
            fs::remove_dir_all(disk.root.join("histories")).unwrap();
        }
        let engine = Engine::open(&root).unwrap();
        assert_eq!(
            engine.operation(&operation).unwrap().commit_state,
            "PUBLISHED"
        );
        assert_eq!(engine.list_tree(&branch).unwrap().len(), 1);
        assert!(engine.check().unwrap().issues.is_empty());
    }
}
