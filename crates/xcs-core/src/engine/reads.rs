//! Verified replicas, read leases, offline export and disk observations.
use super::{Engine, OpenedContent, ReadLease};
use crate::{disk::*, journal::verify_local_publication, *};
use std::path::Path;
use std::{io::Read, path::PathBuf, sync::Arc};

impl Engine {
    pub(crate) fn used_bytes(&self, slot: &str) -> u64 {
        self.state
            .placements
            .values()
            .filter(|p| p.route_slot_ids.iter().any(|s| s == slot))
            .filter(|p| {
                self.state
                    .content_states
                    .get(&p.version_id)
                    .is_none_or(|s| s != "PURGED")
            })
            .filter_map(|p| self.state.versions.get(&p.version_id))
            .fold(0u64, |sum, v| sum.saturating_add(v.logical_size))
    }
    pub fn disk_status(&self) -> Result<Vec<DiskStatus>> {
        Ok(self
            .config
            .disks
            .iter()
            .map(|d| {
                let seal = validate_seal(&self.config, d).ok();
                DiskStatus {
                    slot_id: d.slot_id.clone(),
                    disk_id: d.disk_id.clone(),
                    level: d.level,
                    upstream_slot_id: d.upstream_slot_id.clone(),
                    online: seal.is_some(),
                    metadata_member: d.metadata_member,
                    physical_device: seal.map(|s| s.physical_device).unwrap_or_default(),
                    available_bytes: available(&d.root).unwrap_or(0),
                    used_bytes: self.used_bytes(&d.slot_id),
                    simulation: self.config.mode == Mode::Development,
                }
            })
            .collect())
    }
    /// Filesystem capacity is sampled from the actual managed member mount.
    /// Development directories may share one physical filesystem.
    pub fn disk_capacity(&self, slot: &str) -> Result<(u64, u64, u64)> {
        let disk = self.disk(slot)?;
        let stat = rustix::fs::statvfs(&disk.root).map_err(std::io::Error::from)?;
        let total = stat.f_blocks.saturating_mul(stat.f_frsize);
        let available = stat.f_bavail.saturating_mul(stat.f_frsize);
        let used = stat
            .f_blocks
            .saturating_sub(stat.f_bfree)
            .saturating_mul(stat.f_frsize);
        Ok((total, available, used))
    }
    pub(crate) fn valid_replica(&self, version_id: &str) -> Result<(PathBuf, FileVersion)> {
        let (content, version, _) = self.verified_replica(version_id)?;
        Ok((content, version))
    }
    fn verified_replica(&self, version_id: &str) -> Result<(PathBuf, FileVersion, PathBuf)> {
        if self
            .state
            .content_states
            .get(version_id)
            .is_some_and(|s| s != "AVAILABLE")
        {
            return Err(Error::new(
                "CONTENT_PURGED",
                "version content is no longer available in this history",
            ));
        }
        let version = self
            .state
            .versions
            .get(version_id)
            .ok_or_else(|| Error::new("VERSION_NOT_FOUND", "version does not exist"))?;
        let placement = self
            .state
            .placements
            .get(version_id)
            .ok_or_else(|| Error::new("PLACEMENT_NOT_FOUND", "version placement missing"))?;
        for slot in &placement.route_slot_ids {
            let d = self.disk(slot)?;
            if validate_seal(&self.config, d).is_err() {
                continue;
            }
            let dir = version_dir(d, &version.history_tree_id, version_id);
            if verify_local_publication(&dir, version).is_ok()
                && verify_payload(&dir, version).is_ok()
            {
                return Ok((
                    crate::container_storage::content_path(&dir, version)?,
                    version.clone(),
                    dir,
                ));
            }
        }
        Err(Error::new(
            "CONTENT_UNAVAILABLE",
            "no verified complete replica is available",
        ))
    }
    pub fn open_content(&self, version_id: &str) -> Result<OpenedContent> {
        let (path, version) = self.valid_replica(version_id)?;
        let file = open_regular(&path)?;
        let mut reads = self
            .reads
            .lock()
            .map_err(|_| Error::new("INTERNAL_ERROR", "read lease lock poisoned"))?;
        *reads.entry(version_id.into()).or_default() += 1;
        drop(reads);
        Ok(OpenedContent {
            file,
            size: version.logical_size,
            content_hash: version.content_hash,
            lease: ReadLease {
                version: version_id.into(),
                reads: Arc::clone(&self.reads),
            },
        })
    }
    pub fn open_workspace_content(
        &self,
        workspace: &str,
        item: &str,
        principal: &str,
    ) -> Result<OpenedContent> {
        let w = self.workspace(workspace)?;
        if w.principal != principal {
            return Err(Error::new(
                "PERMISSION_DENIED",
                "workspace belongs to another principal",
            ));
        }
        self.require_grant(self.repository_for_branch(&w.branch_id)?, principal, "read")?;
        let staged = w
            .files
            .get(item)
            .ok_or_else(|| Error::new("ITEM_NOT_FOUND", "workspace item not found"))?;
        let mut file = open_regular(&staged.source_path)?;
        let mut h = blake3::Hasher::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        if h.finalize().to_hex().as_str() != staged.content_hash {
            return Err(Error::new(
                "CHECKSUM_MISMATCH",
                "workspace full file was modified outside the engine",
            ));
        }
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0))?;
        let key = staged
            .base_version_id
            .clone()
            .unwrap_or_else(|| format!("workspace:{workspace}:{item}"));
        *self
            .reads
            .lock()
            .map_err(|_| Error::new("INTERNAL_ERROR", "read lease lock poisoned"))?
            .entry(key.clone())
            .or_default() += 1;
        Ok(OpenedContent {
            file,
            size: staged.size,
            content_hash: staged.content_hash.clone(),
            lease: ReadLease {
                version: key,
                reads: Arc::clone(&self.reads),
            },
        })
    }
    pub fn read_checksums(&self, version: &str) -> Result<Vec<u8>> {
        let (_, _, directory) = self.verified_replica(version)?;
        read_bytes(&directory.join("checksums.bin"), MAX_CHECKSUM_MANIFEST)
    }
    pub fn export_version(&self, version_id: &str, destination: &Path) -> Result<()> {
        let (path, version) = self.valid_replica(version_id)?;
        full_copy(
            &path,
            destination,
            version.checksum_chunk_bytes,
            version.logical_size,
        )?;
        Ok(())
    }
    pub fn check(&self) -> Result<CheckReport> {
        let mut result = CheckReport {
            checked_versions: 0,
            healthy_replicas: 0,
            tombstoned_versions: 0,
            issues: Vec::new(),
        };
        for d in &self.config.disks {
            match offline_check(&d.root) {
                Ok(r) => {
                    result.checked_versions += r.checked_versions;
                    result.healthy_replicas += r.healthy_replicas;
                    result.tombstoned_versions += r.tombstoned_versions;
                    result
                        .issues
                        .extend(r.issues.into_iter().map(|s| format!("{}: {s}", d.slot_id)));
                }
                Err(e) => result.issues.push(format!("{}: {e}", d.slot_id)),
            }
        }
        Ok(result)
    }
}
