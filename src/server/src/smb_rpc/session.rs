//! Per-user session, namespace dispatch and writer-lock ownership.
use super::*;

pub(super) struct Session {
    pub(super) shared: Arc<Shared>,
    pub(super) identity: Identity,
    pub(super) branch: String,
    pub(super) handles: BTreeMap<u64, Handle>,
    pub(super) next_handle: u64,
    pub(super) writer_locks: BTreeSet<String>,
    pub(super) max_file_bytes: u64,
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Ok(mut writers) = self.shared.file_writers.lock() {
            for path in &self.writer_locks {
                writers.remove(&(self.branch.clone(), path.clone()));
            }
        }
        // Every live writer has handed out a mutable FD. The peer can have
        // changed it before a Dirty request reached us, including after a
        // successful Flush. A disconnect therefore cannot prove clean bytes.
        // Retain both data and the recovery record without hashing in Drop or
        // publishing a new generation. Explicit successful Close still cleans up.
    }
}

impl Session {
    pub(super) fn call(
        &self,
        action: crate::BackendAction,
        input: Value,
    ) -> Result<Value, ApiError> {
        self.shared.backend.call(action, &self.identity, input)
    }
    pub(super) fn entries(&self) -> Result<Vec<Entry>, ApiError> {
        let tree = self.call(crate::BackendAction::Tree, json!({"branch_id":self.branch}))?;
        serde_json::from_value(
            tree.get("entries")
                .cloned()
                .ok_or_else(|| ApiError::invalid("missing tree entries"))?,
        )
        .map_err(ApiError::invalid)
    }
    pub(super) fn find(&self, path: &str) -> Result<Option<Entry>, ApiError> {
        let key = name_policy::path_key(path);
        Ok(self
            .entries()?
            .into_iter()
            .find(|entry| name_policy::path_key(&entry.path) == key))
    }
    pub(super) fn workspace(&self) -> Result<Workspace, ApiError> {
        serde_json::from_value(self.call(
            crate::BackendAction::CreateWorkspace,
            json!({"branch_id":self.branch}),
        )?)
        .map_err(ApiError::invalid)
    }
    pub(super) fn reserve_paths(&mut self, paths: &[&str]) -> Result<Vec<String>, ApiError> {
        let keys = paths
            .iter()
            .map(|path| name_policy::path_key(path))
            .collect::<BTreeSet<_>>();
        let overlaps = |left: &str, right: &str| {
            left == right
                || left.starts_with(&format!("{right}/"))
                || right.starts_with(&format!("{left}/"))
        };
        let mut writers = self
            .shared
            .file_writers
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "SMB lock registry poisoned"))?;
        if writers.iter().any(|(branch, existing)| {
            branch == &self.branch && keys.iter().any(|key| overlaps(existing, key))
        }) {
            return Err(ApiError::conflict(
                "FILE_WRITE_BUSY",
                "an affected SMB file has an active write handle",
            ));
        }
        for key in &keys {
            writers.insert((self.branch.clone(), key.clone()));
            self.writer_locks.insert(key.clone());
        }
        Ok(keys.into_iter().collect())
    }
    pub(super) fn release_paths(&mut self, paths: &[String]) {
        if let Ok(mut writers) = self.shared.file_writers.lock() {
            for path in paths {
                writers.remove(&(self.branch.clone(), path.clone()));
                self.writer_locks.remove(path);
            }
        }
    }
    pub(super) fn dispatch(
        &mut self,
        request: Request,
    ) -> Result<(Value, Option<OwnedFd>), ApiError> {
        let value = match request {
            Request::Hello { .. } => return Err(ApiError::invalid("hello is allowed only once")),
            Request::Open {
                path,
                writable,
                create,
                truncate,
                exclusive,
                write_through,
            } => return self.open(path, writable, create, truncate, exclusive, write_through),
            Request::AuthorizeRead { handle } => self.authorize_read(handle)?,
            Request::Dirty { handle } => self.dirty(handle)?,
            Request::Flush { handle } => self.flush(handle)?,
            Request::Close { handle } => self.close(handle, false)?,
            Request::Abort { handle } => self.close(handle, true)?,
            Request::Stat { path } => {
                if path.is_empty() || path == "." {
                    json!({"path":"","kind":"directory","size":"0","entry_id":self.branch})
                } else {
                    name_policy::validate_path(&path).map_err(contract)?;
                    serde_json::to_value(
                        self.find(&path)?
                            .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "entry missing"))?,
                    )
                    .map_err(ApiError::invalid)?
                }
            }
            Request::List { path } => {
                if !path.is_empty() {
                    name_policy::validate_path(&path).map_err(contract)?;
                }
                let entries = self.entries()?;
                let key = name_policy::path_key(&path);
                if !path.is_empty() {
                    let directory = entries
                        .iter()
                        .find(|entry| entry.lookup_key == key)
                        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "directory missing"))?;
                    if directory.kind != "directory" {
                        return Err(ApiError::new(
                            409,
                            "NOT_DIRECTORY",
                            "entry is not a directory",
                        ));
                    }
                }
                let prefix = if key.is_empty() {
                    String::new()
                } else {
                    format!("{key}/")
                };
                let entries = entries
                    .into_iter()
                    .filter(|entry| {
                        entry
                            .lookup_key
                            .strip_prefix(&prefix)
                            .is_some_and(|rest| !rest.contains('/'))
                    })
                    .collect::<Vec<_>>();
                json!({"entries":entries})
            }
            Request::Namespace {
                kind,
                path,
                destination,
                replace,
            } => {
                if !["mkdir", "rename", "delete"].contains(&kind.as_str()) {
                    return Err(ApiError::unsupported());
                }
                let mut affected = vec![path.as_str()];
                if let Some(target) = destination.as_deref() {
                    affected.push(target);
                }
                let reserved = self.reserve_paths(&affected)?;
                let mut pending_workspace = None;
                let result = (|| {
                    let workspace = self.workspace()?;
                    pending_workspace = Some(workspace.workspace_id.clone());
                    let entries = self.entries()?;
                    let nonempty = |entry: &Entry| {
                        let prefix = format!("{}/", entry.lookup_key);
                        entry.kind == "directory"
                            && entries
                                .iter()
                                .any(|child| child.lookup_key.starts_with(&prefix))
                    };
                    if kind == "delete" && self.find(&path)?.as_ref().is_some_and(nonempty) {
                        return Err(ApiError::new(
                            409,
                            "DIRECTORY_NOT_EMPTY",
                            "SMB directory removal requires an empty directory",
                        ));
                    }
                    if kind == "rename"
                        && replace
                        && let Some(target) = destination.as_deref()
                        && let Some(target_entry) = self.find(target)?
                    {
                        let source = self.find(&path)?.ok_or_else(|| {
                            ApiError::new(404, "NOT_FOUND", "rename source missing")
                        })?;
                        if source.kind != target_entry.kind {
                            return Err(ApiError::invalid(
                                "rename replacement requires matching entry kinds",
                            ));
                        }
                        // A case-only rename addresses the same stable entry.
                        if source.entry_id != target_entry.entry_id {
                            if nonempty(&target_entry) {
                                return Err(ApiError::new(
                                    409,
                                    "DIRECTORY_NOT_EMPTY",
                                    "rename replacement directory is not empty",
                                ));
                            }
                            self.call(crate::BackendAction::Namespace,json!({"workspace_id":workspace.workspace_id,"kind":"delete","path":target,"destination":null}))?;
                        }
                    }
                    self.call(crate::BackendAction::Namespace,json!({"workspace_id":workspace.workspace_id,"kind":kind,"path":path,"destination":destination}))?;
                    self.call(crate::BackendAction::CommitWorkspaceInternal,json!({"workspace_id":workspace.workspace_id,"operation_id":new_id("op"),"expected_head":workspace.base_head,"fencing_token":workspace.fencing_token.to_string(),"message":"SMB namespace change"}))
                })();
                if result.is_err()
                    && let Some(id) = pending_workspace
                {
                    let _ = self.call(
                        crate::BackendAction::AbortWorkspace,
                        json!({"workspace_id":id}),
                    );
                }
                self.release_paths(&reserved);
                result?
            }
        };
        Ok((value, None))
    }
}
