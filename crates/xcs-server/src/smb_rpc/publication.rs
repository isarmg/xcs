//! Dirty generations, recovery records and publication acknowledgement.
use super::*;

pub(super) fn recovery_record(
    writer: &Writer,
    principal: &str,
    branch: &str,
) -> Result<(), ApiError> {
    let parent = writer
        .source
        .parent()
        .ok_or_else(|| ApiError::invalid("SMB staging parent missing"))?;
    let value = json!({"format":"xcs-smb-work-v1","principal":principal,"branch_id":branch,"path":writer.path,"item_id":writer.item_id,"source_path":writer.source,"workspace_id":writer.workspace.workspace_id,"expected_head":writer.workspace.base_head,"fencing_token":writer.workspace.fencing_token.to_string(),"operation_id":writer.operation_id,"dirty":writer.dirty,"published":writer.committed});
    let mut temporary = tempfile::Builder::new()
        .prefix(".smb-work-")
        .tempfile_in(parent)
        .map_err(ApiError::io)?;
    temporary
        .write_all(&serde_json::to_vec(&value).map_err(ApiError::invalid)?)
        .map_err(ApiError::io)?;
    temporary.as_file().sync_all().map_err(ApiError::io)?;
    temporary
        .persist(writer.source.with_extension("work.json"))
        .map_err(|error| ApiError::io(error.error))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(ApiError::io)
}

pub(super) fn cleanup_writer(writer: &Writer) {
    for path in [&writer.source, &writer.source.with_extension("work.json")] {
        if let Err(error) = fs::remove_file(path) {
            if error.kind() != io::ErrorKind::NotFound {
                tracing::warn!(%error,path=%path.display(),"SMB staging cleanup failed");
            }
        }
    }
}

pub(super) fn finish_publication(
    shared: &Shared,
    identity: &Identity,
    branch: &str,
    writer: &mut Writer,
    result: Value,
) -> Result<Value, ApiError> {
    writer.last_head = field(result.get("result").unwrap_or(&result), "commit_id")?.to_owned();
    writer.committed = true;
    // Bytes may have changed after a response was lost. Never acknowledge the
    // earlier publication as if it included those later FD-backed writes.
    let tree = shared.backend.call(
        "tree",
        identity,
        json!({"branch_id":branch,"query":{"commit_id":writer.last_head}}),
    )?;
    let entries: Vec<Entry> =
        serde_json::from_value(tree["entries"].clone()).map_err(ApiError::invalid)?;
    let entry = entries
        .iter()
        .find(|entry| entry.entry_id == writer.item_id)
        .ok_or_else(|| {
            ApiError::conflict("FILE_VERSION_CONFLICT", "published file identity missing")
        })?;
    let published = shared.backend.content(identity, &entry.version_id)?;
    let mut hash = blake3::Hasher::new();
    let mut offset = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        let length = writer
            .file
            .read_at(&mut buffer, offset)
            .map_err(ApiError::io)?;
        if length == 0 {
            break;
        }
        hash.update(&buffer[..length]);
        offset = offset
            .checked_add(length as u64)
            .ok_or_else(|| ApiError::limit("file byte count overflow"))?;
    }
    writer.dirty = offset != published.size || hash.finalize().to_hex().as_str() != published.hash;
    if let Err(error) = recovery_record(writer, &identity.principal, branch) {
        tracing::warn!(%error,"published SMB recovery record could not be refreshed");
    }
    if writer.dirty {
        return Err(ApiError::conflict(
            "WORKSPACE_REBASE_REQUIRED",
            "original save published; later changed bytes are retained and need a new dirty request",
        ));
    }
    Ok(result)
}

impl Session {
    pub(super) fn dirty(&mut self, handle: u64) -> Result<Value, ApiError> {
        // The FD has already been modified. Preserve it even when current
        // authorization or rebase validation rejects the operation.
        let (needs_workspace, path, last_head, captured_workspace) =
            match self.handles.get_mut(&handle) {
                Some(Handle::Write(writer)) => {
                    writer.dirty = true;
                    recovery_record(writer, &self.identity.principal, &self.branch)?;
                    (
                        writer.committed,
                        writer.path.clone(),
                        writer.last_head.clone(),
                        writer.workspace.workspace_id.clone(),
                    )
                }
                _ => return Err(ApiError::forbidden()),
            };
        self.call("tree", json!({"branch_id":self.branch}))?;
        // Even an already published handle remains bound to the selection it
        // opened. A same-content default switch must not retarget this FD.
        self.call(
            "workspace_selection_authorize",
            json!({
                "workspace_id":captured_workspace,"path":path
            }),
        )?;
        let new_workspace = if needs_workspace {
            let workspace = self.workspace()?;
            let validate = (|| {
                let original = self.call(
                    "tree",
                    json!({"branch_id":self.branch,"query":{"commit_id":last_head}}),
                )?;
                let current = self.call(
                    "tree",
                    json!({"branch_id":self.branch,"query":{"commit_id":workspace.base_head}}),
                )?;
                let select = |tree: &Value| -> Result<Option<Entry>, ApiError> {
                    let entries: Vec<Entry> = serde_json::from_value(tree["entries"].clone())
                        .map_err(ApiError::invalid)?;
                    Ok(entries
                        .into_iter()
                        .find(|entry| entry.lookup_key == xcs_naming::path_key(&path)))
                };
                let previous = select(&original)?;
                let current = select(&current)?;
                if previous
                    .as_ref()
                    .map(|e| (&e.entry_id, &e.version_id, &e.path))
                    != current
                        .as_ref()
                        .map(|e| (&e.entry_id, &e.version_id, &e.path))
                {
                    return Err(ApiError::conflict(
                        "FILE_VERSION_CONFLICT",
                        "the open file changed since this handle last published",
                    ));
                }
                Ok(())
            })();
            if let Err(error) = validate {
                let _ = self.call(
                    "abort_workspace",
                    json!({"workspace_id":workspace.workspace_id}),
                );
                return Err(error);
            }
            Some(workspace)
        } else {
            if let Some(Handle::Write(writer)) = self.handles.get(&handle) {
                self.call(
                    "upload_authorize",
                    json!({"workspace_id":writer.workspace.workspace_id,"item_id":writer.item_id}),
                )?;
            }
            None
        };
        let Some(Handle::Write(writer)) = self.handles.get_mut(&handle) else {
            return Err(ApiError::forbidden());
        };
        if let Some(workspace) = new_workspace {
            writer.workspace = workspace;
            writer.committed = false;
            writer.operation_id = new_id("op");
        }
        recovery_record(writer, &self.identity.principal, &self.branch)?;
        if writer.file.metadata().map_err(ApiError::io)?.len() > self.max_file_bytes {
            return Err(ApiError::limit("file exceeds SMB byte limit"));
        }
        if writer.write_through {
            self.flush(handle)
        } else {
            Ok(json!({"dirty":true}))
        }
    }
    pub(super) fn flush(&mut self, handle: u64) -> Result<Value, ApiError> {
        let Some(entry) = self.handles.get_mut(&handle) else {
            return Err(ApiError::new(404, "NOT_FOUND", "SMB handle missing"));
        };
        let writer = match entry {
            Handle::Read(content, _) => {
                content.file.sync_all().map_err(ApiError::io)?;
                return Ok(json!({"saved":true,"changed":false}));
            }
            Handle::Directory(file) => {
                file.sync_all().map_err(ApiError::io)?;
                return Ok(json!({"saved":true,"changed":false}));
            }
            Handle::Write(writer) => writer,
        };
        if !writer.dirty {
            return Ok(json!({"saved":true,"changed":false}));
        }
        if writer.committed {
            return Err(ApiError::conflict(
                "WORKSPACE_REBASE_REQUIRED",
                "changed bytes need an authorized workspace before saving",
            ));
        }
        // A lost response must not restage under a new dirty generation and
        // thereby change the original request digest. Reconcile by operation id.
        match self.shared.backend.call(
            "operation",
            &self.identity,
            json!({"operation_id":writer.operation_id}),
        ) {
            Ok(result)
                if result.get("commit_state").and_then(Value::as_str) == Some("PUBLISHED") =>
            {
                return finish_publication(
                    &self.shared,
                    &self.identity,
                    &self.branch,
                    writer,
                    result,
                );
            }
            Ok(_) => {
                return Err(ApiError::new(
                    503,
                    "RECOVERY_REQUIRED",
                    "SMB save outcome must be reconciled before retrying",
                ));
            }
            Err(error) if error.code == "NOT_FOUND" || error.code == "OPERATION_NOT_FOUND" => {}
            Err(error) => return Err(error),
        }
        writer.file.sync_all().map_err(ApiError::io)?;
        if writer.file.metadata().map_err(ApiError::io)?.len() > self.max_file_bytes {
            return Err(ApiError::limit("file exceeds SMB byte limit"));
        }
        self.shared.backend.upload(
            &self.identity,
            &writer.workspace.workspace_id,
            &writer.item_id,
            &writer.path,
            &writer.source,
        )?;
        let result=self.shared.backend.call("commit_workspace_internal",&self.identity,json!({"workspace_id":writer.workspace.workspace_id,"operation_id":writer.operation_id,"expected_head":writer.workspace.base_head,"fencing_token":writer.workspace.fencing_token.to_string(),"message":"SMB full-file save"}))?;
        finish_publication(&self.shared, &self.identity, &self.branch, writer, result)
    }
    pub(super) fn close(&mut self, handle: u64, abort: bool) -> Result<Value, ApiError> {
        if !self.handles.contains_key(&handle) {
            return Err(ApiError::new(404, "NOT_FOUND", "SMB handle missing"));
        }
        if !abort {
            self.flush(handle)?;
        }
        if let Some(Handle::Write(writer)) = self.handles.get(&handle) {
            if !writer.committed {
                self.call(
                    "abort_workspace",
                    json!({"workspace_id":writer.workspace.workspace_id}),
                )?;
            }
        }
        if let Some(Handle::Write(writer)) = self.handles.remove(&handle) {
            cleanup_writer(&writer);
            self.release_paths(&[xcs_naming::path_key(&writer.path)]);
        }
        Ok(json!({"closed":true,"aborted":abort}))
    }
}
