//! Descriptor creation, immutable reads and handle admission.
use super::*;

impl Session {
    pub(super) fn open(
        &mut self,
        path: String,
        writable: bool,
        create: bool,
        truncate: bool,
        exclusive: bool,
        write_through: bool,
    ) -> Result<(Value, Option<OwnedFd>), ApiError> {
        if !path.is_empty() {
            name_policy::validate_path(&path).map_err(contract)?;
        }
        if self.handles.len() >= MAX_HANDLES {
            return Err(ApiError::limit("too many SMB handles"));
        }
        let existing = if path.is_empty() {
            None
        } else {
            self.find(&path)?
        };
        if existing.is_some() && create && exclusive {
            return Err(ApiError::conflict("NAME_CONFLICT", "entry already exists"));
        }
        if existing.is_none() && !create && !path.is_empty() {
            return Err(ApiError::new(404, "NOT_FOUND", "entry does not exist"));
        }
        let handle_id = self.next_handle;
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .ok_or_else(|| ApiError::limit("SMB handle identifiers exhausted"))?;
        // An unwritten branch reads its parent's immutable version. A private
        // write copy starts from those visible bytes, while merely opening and
        // closing the handle leaves the branch without an own version.
        let version = existing
            .as_ref()
            .filter(|entry| !entry.version_id.is_empty())
            .map(|entry| entry.version_id.clone());
        let mut entry_id = existing
            .as_ref()
            .map(|entry| entry.entry_id.clone())
            .unwrap_or_else(|| self.branch.clone());
        if path.is_empty()
            || existing
                .as_ref()
                .is_some_and(|entry| entry.kind == "directory")
        {
            if writable || create || truncate {
                return Err(ApiError::new(
                    409,
                    "IS_DIRECTORY",
                    "directory data is not writable",
                ));
            }
            let file =
                File::open(self.shared.spool.join("empty-directory")).map_err(ApiError::io)?;
            let fd = OwnedFd::from(file.try_clone().map_err(ApiError::io)?);
            self.handles.insert(handle_id, Handle::Directory(file));
            let result = OpenResult {
                entry_id,
                kind: "directory".into(),
                path,
                handle: handle_id,
                size: 0,
                max_file_bytes: self.max_file_bytes,
                version_id: None,
                workspace_id: None,
            };
            return Ok((
                serde_json::to_value(result).map_err(ApiError::invalid)?,
                Some(fd),
            ));
        }
        let (handle, fd, size, workspace_id) = if writable {
            self.reserve_paths(&[path.as_str()])?;
            let mut pending_workspace = None;
            let mut pending_source = None;
            let result = (|| {
                let workspace = self.workspace()?;
                pending_workspace = Some(workspace.workspace_id.clone());
                // Bind the observed file to the workspace's immutable baseline.
                // An HTTP writer can commit between find() and workspace creation,
                // even while this session owns the SMB-only path reservation.
                let snapshot = self.call(
                    crate::BackendAction::Tree,
                    json!({
                        "branch_id":self.branch,"query":{"commit_id":workspace.base_head}
                    }),
                )?;
                let entries: Vec<Entry> = serde_json::from_value(snapshot["entries"].clone())
                    .map_err(ApiError::invalid)?;
                let baseline = entries
                    .iter()
                    .find(|entry| entry.lookup_key == name_policy::path_key(&path));
                let identity = |entry: &Entry| {
                    (
                        entry.entry_id.clone(),
                        entry.version_id.clone(),
                        entry.path.clone(),
                        entry.kind.clone(),
                    )
                };
                if existing.as_ref().map(identity) != baseline.map(identity) {
                    return Err(ApiError::conflict(
                        "FILE_VERSION_CONFLICT",
                        "the file changed while its write handle was opening; reopen it",
                    ));
                }
                let temp = tempfile::Builder::new()
                    .prefix(&format!("{}-", workspace.workspace_id))
                    .tempfile_in(&self.shared.spool)
                    .map_err(ApiError::io)?;
                let (mut file, source) = temp.keep().map_err(|error| ApiError::io(error.error))?;
                pending_source = Some(source.clone());
                if let Some(version) = version.as_deref().filter(|_| !truncate) {
                    let mut content = self.shared.backend.content(&self.identity, version)?;
                    let mut buffer = [0u8; 65536];
                    loop {
                        let length = content.file.read(&mut buffer).map_err(ApiError::io)?;
                        if length == 0 {
                            break;
                        }
                        file.write_all(&buffer[..length]).map_err(ApiError::io)?;
                    }
                }
                let size = file.metadata().map_err(ApiError::io)?.len();
                if size > self.max_file_bytes {
                    return Err(ApiError::limit("file exceeds SMB byte limit"));
                }
                if existing.is_none() {
                    // Allocate the future entry identity in this unpublished
                    // workspace so the SMB file id remains stable at first save.
                    entry_id = new_id("entry");
                    let staged = self.shared.backend.upload(
                        &self.identity,
                        &workspace.workspace_id,
                        &entry_id,
                        &path,
                        &source,
                    )?;
                    entry_id = field(&staged, "item_id")?.to_owned();
                }
                let fd = OwnedFd::from(file.try_clone().map_err(ApiError::io)?);
                let workspace_id = workspace.workspace_id.clone();
                let writer = Writer {
                    file,
                    source,
                    path: path.clone(),
                    item_id: entry_id.clone(),
                    last_head: workspace.base_head.clone(),
                    workspace,
                    dirty: existing.is_none() || truncate,
                    committed: false,
                    operation_id: new_id("op"),
                    write_through,
                };
                writer.file.sync_all().map_err(ApiError::io)?;
                recovery_record(&writer, &self.identity.principal, &self.branch)?;
                Ok((
                    Handle::Write(Box::new(writer)),
                    fd,
                    size,
                    Some(workspace_id),
                ))
            })();
            if result.is_err() {
                if let Some(id) = pending_workspace {
                    let _ = self.call(
                        crate::BackendAction::AbortWorkspace,
                        json!({"workspace_id":id}),
                    );
                }
                if let Some(path) = pending_source {
                    let _ = fs::remove_file(path.with_extension("work.json"));
                    let _ = fs::remove_file(path);
                }
                self.release_paths(&[name_policy::path_key(&path)]);
            }
            result?
        } else {
            if create || truncate {
                return Err(ApiError::invalid("read handle cannot create or truncate"));
            }
            let content = self.shared.backend.content(
                &self.identity,
                version
                    .as_deref()
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "version missing"))?,
            )?;
            let size = content.size;
            let fd = OwnedFd::from(content.file.try_clone().map_err(ApiError::io)?);
            (
                Handle::Read(
                    content,
                    version
                        .clone()
                        .ok_or_else(|| ApiError::invalid("read version missing"))?,
                ),
                fd,
                size,
                None,
            )
        };
        self.handles.insert(handle_id, handle);
        let result = OpenResult {
            entry_id,
            kind: "file".into(),
            path,
            handle: handle_id,
            size,
            max_file_bytes: self.max_file_bytes,
            version_id: version,
            workspace_id,
        };
        Ok((
            serde_json::to_value(result).map_err(ApiError::invalid)?,
            Some(fd),
        ))
    }
    pub(super) fn authorize_read(&self, handle: u64) -> Result<Value, ApiError> {
        match self.handles.get(&handle) {
            Some(Handle::Read(_, version)) => {
                self.call(crate::BackendAction::Version, json!({"version_id":version}))?;
            }
            Some(Handle::Write(_)) | Some(Handle::Directory(_)) => {
                self.call(crate::BackendAction::Tree, json!({"branch_id":self.branch}))?;
            }
            None => return Err(ApiError::new(404, "NOT_FOUND", "SMB handle missing")),
        }
        Ok(json!({"authorized":true}))
    }
}
