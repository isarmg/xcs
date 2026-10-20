//! Protected local Samba bridge. Every operation is authorized by Backend;
//! mutable files remain unpublished until the normal core commit succeeds.
use crate::{ApiError, Backend, Content, auth::Identity};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read, Write},
    os::{
        fd::{AsFd, OwnedFd},
        unix::fs::{FileExt, MetadataExt, PermissionsExt},
    },
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};
use xcs_contract::{Entry, Workspace, new_id, parse_u64_decimal};
use xcs_smb::protocol::{FrameStream, OpenResult, PROTOCOL, Request, Response};

const MAX_CONNECTIONS: usize = 64;
const MAX_HANDLES: usize = 1024;

struct SocketGuard {
    path: PathBuf,
    identity: (u64, u64),
    connections: Arc<Mutex<BTreeMap<u64, std::os::unix::net::UnixStream>>>,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if let Ok(mut connections) = self.connections.lock() {
            for stream in connections.values() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
            connections.clear();
        }
        if let Ok(metadata) = fs::symlink_metadata(&self.path) {
            if (metadata.dev(), metadata.ino()) == self.identity {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}
struct ConnectionGuard {
    id: u64,
    connections: Arc<Mutex<BTreeMap<u64, std::os::unix::net::UnixStream>>>,
}
impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if let Ok(mut connections) = self.connections.lock() {
            connections.remove(&self.id);
        }
    }
}

struct Shared {
    backend: Arc<dyn Backend>,
    identities: BTreeMap<String, Identity>,
    spool: PathBuf,
    file_writers: Mutex<BTreeSet<(String, String)>>,
}

enum Handle {
    Read(Content, String),
    Directory(File),
    Write(Writer),
}

struct Writer {
    file: File,
    source: PathBuf,
    path: String,
    item_id: String,
    workspace: Workspace,
    dirty: bool,
    committed: bool,
    operation_id: String,
    last_head: String,
    write_through: bool,
}

fn recovery_record(writer: &Writer, principal: &str, branch: &str) -> Result<(), ApiError> {
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

fn cleanup_writer(writer: &Writer) {
    for path in [&writer.source, &writer.source.with_extension("work.json")] {
        if let Err(error) = fs::remove_file(path) {
            if error.kind() != io::ErrorKind::NotFound {
                tracing::warn!(%error,path=%path.display(),"SMB staging cleanup failed");
            }
        }
    }
}

fn finish_publication(
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

struct Session {
    shared: Arc<Shared>,
    identity: Identity,
    branch: String,
    handles: BTreeMap<u64, Handle>,
    next_handle: u64,
    writer_locks: BTreeSet<String>,
    max_file_bytes: u64,
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Ok(mut writers) = self.shared.file_writers.lock() {
            for path in &self.writer_locks {
                writers.remove(&(self.branch.clone(), path.clone()));
            }
        }
        // Dirty writer source files are deliberately retained on a disconnect.
        // The client did not receive a successful save. Operators can recover
        // these private files; silently committing or deleting them is forbidden.
        for handle in self.handles.values() {
            if let Handle::Write(writer) = handle {
                if !writer.dirty {
                    cleanup_writer(writer);
                    if !writer.committed {
                        let _ = self.shared.backend.call(
                            "abort_workspace",
                            &self.identity,
                            json!({"workspace_id":writer.workspace.workspace_id}),
                        );
                    }
                }
            }
        }
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::invalid(format!("missing {key}")))
}
fn contract(error: xcs_contract::Error) -> ApiError {
    ApiError::invalid(error)
}

impl Session {
    fn call(&self, action: &str, input: Value) -> Result<Value, ApiError> {
        self.shared.backend.call(action, &self.identity, input)
    }
    fn entries(&self) -> Result<Vec<Entry>, ApiError> {
        let tree = self.call("tree", json!({"branch_id":self.branch}))?;
        serde_json::from_value(
            tree.get("entries")
                .cloned()
                .ok_or_else(|| ApiError::invalid("missing tree entries"))?,
        )
        .map_err(ApiError::invalid)
    }
    fn find(&self, path: &str) -> Result<Option<Entry>, ApiError> {
        let key = xcs_naming::path_key(path);
        Ok(self
            .entries()?
            .into_iter()
            .find(|entry| xcs_naming::path_key(&entry.path) == key))
    }
    fn workspace(&self) -> Result<Workspace, ApiError> {
        serde_json::from_value(self.call("create_workspace", json!({"branch_id":self.branch}))?)
            .map_err(ApiError::invalid)
    }
    fn reserve_paths(&mut self, paths: &[&str]) -> Result<Vec<String>, ApiError> {
        let keys = paths
            .iter()
            .map(|path| xcs_naming::path_key(path))
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
    fn release_paths(&mut self, paths: &[String]) {
        if let Ok(mut writers) = self.shared.file_writers.lock() {
            for path in paths {
                writers.remove(&(self.branch.clone(), path.clone()));
                self.writer_locks.remove(path);
            }
        }
    }
    fn open(
        &mut self,
        path: String,
        writable: bool,
        create: bool,
        truncate: bool,
        exclusive: bool,
        write_through: bool,
    ) -> Result<(Value, Option<OwnedFd>), ApiError> {
        if !path.is_empty() {
            xcs_naming::validate_path(&path).map_err(contract)?;
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
                Ok((Handle::Write(writer), fd, size, Some(workspace_id)))
            })();
            if result.is_err() {
                if let Some(id) = pending_workspace {
                    let _ = self.call("abort_workspace", json!({"workspace_id":id}));
                }
                if let Some(path) = pending_source {
                    let _ = fs::remove_file(path.with_extension("work.json"));
                    let _ = fs::remove_file(path);
                }
                self.release_paths(&[xcs_naming::path_key(&path)]);
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
    fn authorize_read(&self, handle: u64) -> Result<Value, ApiError> {
        match self.handles.get(&handle) {
            Some(Handle::Read(_, version)) => {
                self.call("version", json!({"version_id":version}))?;
            }
            Some(Handle::Write(_)) | Some(Handle::Directory(_)) => {
                self.call("tree", json!({"branch_id":self.branch}))?;
            }
            None => return Err(ApiError::new(404, "NOT_FOUND", "SMB handle missing")),
        }
        Ok(json!({"authorized":true}))
    }
    fn dirty(&mut self, handle: u64) -> Result<Value, ApiError> {
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
    fn flush(&mut self, handle: u64) -> Result<Value, ApiError> {
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
    fn close(&mut self, handle: u64, abort: bool) -> Result<Value, ApiError> {
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
    fn dispatch(&mut self, request: Request) -> Result<(Value, Option<OwnedFd>), ApiError> {
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
                    xcs_naming::validate_path(&path).map_err(contract)?;
                    serde_json::to_value(
                        self.find(&path)?
                            .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "entry missing"))?,
                    )
                    .map_err(ApiError::invalid)?
                }
            }
            Request::List { path } => {
                if !path.is_empty() {
                    xcs_naming::validate_path(&path).map_err(contract)?;
                }
                let entries = self.entries()?;
                let key = xcs_naming::path_key(&path);
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
                    if kind == "delete" {
                        if self.find(&path)?.as_ref().is_some_and(nonempty) {
                            return Err(ApiError::new(
                                409,
                                "DIRECTORY_NOT_EMPTY",
                                "SMB directory removal requires an empty directory",
                            ));
                        }
                    }
                    if kind == "rename" && replace {
                        if let Some(target) = destination.as_deref() {
                            if let Some(target_entry) = self.find(target)? {
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
                                    self.call("namespace",json!({"workspace_id":workspace.workspace_id,"kind":"delete","path":target,"destination":null}))?;
                                }
                            }
                        }
                    }
                    self.call("namespace",json!({"workspace_id":workspace.workspace_id,"kind":kind,"path":path,"destination":destination}))?;
                    self.call("commit_workspace_internal",json!({"workspace_id":workspace.workspace_id,"operation_id":new_id("op"),"expected_head":workspace.base_head,"fencing_token":workspace.fencing_token.to_string(),"message":"SMB namespace change"}))
                })();
                if result.is_err() {
                    if let Some(id) = pending_workspace {
                        let _ = self.call("abort_workspace", json!({"workspace_id":id}));
                    }
                }
                self.release_paths(&reserved);
                result?
            }
        };
        Ok((value, None))
    }
}

fn run_connection(mut stream: FrameStream, shared: Arc<Shared>) -> io::Result<()> {
    let (request, fd) = stream.receive::<Request>()?;
    if fd.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "client descriptors forbidden",
        ));
    }
    let setup = (|| {
        let Request::Hello {
            protocol,
            username,
            repository_id,
        } = request
        else {
            return Err(ApiError::unauthorized());
        };
        if protocol != PROTOCOL {
            return Err(ApiError::invalid("unsupported SMB bridge protocol"));
        }
        let identity = shared
            .identities
            .get(&username)
            .cloned()
            .ok_or_else(ApiError::forbidden)?;
        let repo = shared.backend.call(
            "repository",
            &identity,
            json!({"repository_id":repository_id}),
        )?;
        if repo.get("smb_enabled").and_then(Value::as_bool) == Some(false) {
            return Err(ApiError::new(
                409,
                "SMB_REBIND_REQUIRED",
                "repository requires an explicit SMB binding after history migration",
            ));
        }
        let branch = field(&repo, "main_branch_id")?.to_owned();
        let capabilities = shared.backend.call("capabilities", &identity, json!({}))?;
        let max_file_bytes =
            parse_u64_decimal(field(&capabilities, "max_upload_bytes")?).map_err(contract)?;
        Ok(Session {
            shared: shared.clone(),
            identity,
            branch,
            handles: BTreeMap::new(),
            next_handle: 1,
            writer_locks: BTreeSet::new(),
            max_file_bytes,
        })
    })();
    let mut session = match setup {
        Ok(session) => session,
        Err(error) => {
            stream.send(&response_error(error, None), None)?;
            return Ok(());
        }
    };
    stream.send(&Response::Ok{result:json!({"protocol":PROTOCOL,"principal":session.identity.principal,"main_branch_id":session.branch})},None)?;
    loop {
        let (request, fd) = match stream.receive::<Request>() {
            Ok(request) => request,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(error),
        };
        if fd.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "client descriptors forbidden",
            ));
        }
        let operation_id = match &request {
            Request::Flush { handle } | Request::Close { handle } => {
                match session.handles.get(handle) {
                    Some(Handle::Write(writer)) => Some(writer.operation_id.clone()),
                    _ => None,
                }
            }
            _ => None,
        };
        match session.dispatch(request) {
            Ok((result, fd)) => {
                stream.send(&Response::Ok { result }, fd.as_ref().map(AsFd::as_fd))?
            }
            Err(error) => stream.send(&response_error(error, operation_id), None)?,
        }
    }
}

fn response_error(error: ApiError, operation_id: Option<String>) -> Response {
    Response::Error {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        operation_id,
    }
}

fn private_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute socket parent required",
        ));
    }
    let mut current = PathBuf::from("/");
    for part in path.components() {
        if !matches!(part, Component::Normal(_) | Component::RootDir) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "noncanonical socket directory path",
            ));
        }
        if let Component::Normal(part) = part {
            current.push(part);
            let metadata = fs::symlink_metadata(&current)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "socket path contains an alias",
                ));
            }
        }
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.uid() != xcs_smb::protocol::effective_uid() || metadata.mode() & 0o7777 != 0o700 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "socket parent must be service-owned 0700",
        ));
    }
    Ok(())
}

/// Serve only on an existing service-owned private directory. Existing socket
/// names are never unlinked automatically. The trusted Samba daemon must connect
/// using the configured peer uid before impersonating a remote SMB user.
pub async fn serve(
    socket: &Path,
    allowed_uid: u32,
    identities: BTreeMap<String, Identity>,
    backend: Arc<dyn Backend>,
) -> io::Result<()> {
    if identities.is_empty() || identities.values().any(|identity| identity.admin) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SMB requires explicit non-administrator user mappings",
        ));
    }
    let parent = socket
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket parent missing"))?;
    private_directory(parent)?;
    let spool = parent.join("smb-spool");
    match fs::create_dir(&spool) {
        Ok(()) => fs::set_permissions(&spool, fs::Permissions::from_mode(0o700))?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    };
    private_directory(&spool)?;
    let sentinel = spool.join("empty-directory");
    match fs::create_dir(&sentinel) {
        Ok(()) => fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o700))?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    };
    private_directory(&sentinel)?;
    let listener = tokio::net::UnixListener::bind(socket)?;
    let metadata = fs::symlink_metadata(socket)?;
    let connections = Arc::new(Mutex::new(BTreeMap::new()));
    let _socket_guard = SocketGuard {
        path: socket.to_owned(),
        identity: (metadata.dev(), metadata.ino()),
        connections: connections.clone(),
    };
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
    let shared = Arc::new(Shared {
        backend,
        identities,
        spool,
        file_writers: Mutex::new(BTreeSet::new()),
    });
    let admission = Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
    let mut next_connection = 1u64;
    loop {
        let (stream, _) = listener.accept().await?;
        if stream.peer_cred()?.uid() != allowed_uid {
            drop(stream);
            continue;
        }
        let Ok(permit) = admission.clone().try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let stream = stream.into_std()?;
        stream.set_nonblocking(false)?;
        let id = next_connection;
        next_connection = next_connection
            .checked_add(1)
            .ok_or_else(|| io::Error::other("SMB connection identifiers exhausted"))?;
        connections
            .lock()
            .map_err(|_| io::Error::other("SMB connection registry poisoned"))?
            .insert(id, stream.try_clone()?);
        let connection_guard = ConnectionGuard {
            id,
            connections: connections.clone(),
        };
        let shared = shared.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _connection_guard = connection_guard;
            match FrameStream::new(stream).and_then(|stream| run_connection(stream, shared)) {
                Ok(()) => {}
                Err(error) => tracing::warn!(%error,"SMB local RPC session ended"),
            }
        });
    }
}

#[cfg(all(test, feature = "core"))]
mod tests {
    use super::*;
    use crate::engine_backend::EngineBackend;
    use std::{
        os::unix::fs::FileExt,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use xcs_core::{CommitRequest, Config, Engine};

    struct LostCommitReply {
        inner: EngineBackend,
        fail: AtomicBool,
        uploads: AtomicUsize,
        revoked: AtomicBool,
    }
    impl Backend for LostCommitReply {
        fn call(&self, action: &str, identity: &Identity, input: Value) -> Result<Value, ApiError> {
            if action == "version" && self.revoked.load(Ordering::SeqCst) {
                return Err(ApiError::forbidden());
            }
            let value = self.inner.call(action, identity, input)?;
            if action == "commit_workspace_internal" && self.fail.swap(false, Ordering::SeqCst) {
                return Err(ApiError::new(
                    503,
                    "RESPONSE_LOST",
                    "injected after actual durable publication",
                ));
            }
            Ok(value)
        }
        fn upload(
            &self,
            identity: &Identity,
            workspace: &str,
            item: &str,
            path: &str,
            source: &Path,
        ) -> Result<Value, ApiError> {
            self.uploads.fetch_add(1, Ordering::SeqCst);
            self.inner.upload(identity, workspace, item, path, source)
        }
        fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
            self.inner.content(identity, version)
        }
    }

    fn fixture() -> (tempfile::TempDir, Arc<Shared>, Arc<LostCommitReply>, String) {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("p"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&temp.path().join("state"), config).unwrap();
        let repo = engine
            .create_repository("资料", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "alice")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "report.txt", b"old")
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "initial".into(),
                request_digest: None,
            })
            .unwrap();
        let identity = Identity {
            principal: "alice".into(),
            admin: false,
            repositories: BTreeSet::from([repo.repository_id.clone()]),
        };
        let spool = temp.path().join("spool");
        fs::create_dir(&spool).unwrap();
        fs::set_permissions(&spool, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(spool.join("empty-directory")).unwrap();
        let backend = Arc::new(LostCommitReply {
            inner: EngineBackend::new(engine),
            fail: AtomicBool::new(false),
            uploads: AtomicUsize::new(0),
            revoked: AtomicBool::new(false),
        });
        let shared = Arc::new(Shared {
            backend: backend.clone(),
            identities: BTreeMap::from([("alice".into(), identity)]),
            spool,
            file_writers: Mutex::new(BTreeSet::new()),
        });
        (temp, shared, backend, repo.main_branch_id)
    }
    fn session(shared: Arc<Shared>, branch: &str) -> Session {
        Session {
            identity: shared.identities["alice"].clone(),
            shared,
            branch: branch.into(),
            handles: BTreeMap::new(),
            next_handle: 1,
            writer_locks: BTreeSet::new(),
            max_file_bytes: 1 << 30,
        }
    }
    fn open_file(session: &mut Session, writable: bool) -> (OpenResult, File) {
        let (result, fd) = session
            .open("report.txt".into(), writable, false, false, false, false)
            .unwrap();
        (
            serde_json::from_value(result).unwrap(),
            File::from(fd.unwrap()),
        )
    }

    fn select_pending_file(shared: &Shared, branch: &str) -> String {
        let identity = &shared.identities["alice"];
        let tree = shared
            .backend
            .call("tree", identity, json!({"branch_id":branch}))
            .unwrap();
        let entry = tree["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == "report.txt")
            .unwrap();
        let file = entry["file_id"].as_str().unwrap().to_owned();
        let created = shared.backend.call("file_manager_branch", identity, json!({
            "file_id":file,"source_version_id":entry["version_id"],"name":"draft","operation_id":new_id("op")
        })).unwrap();
        shared.backend.call("file_manager_select", identity, json!({
            "file_id":file,"branch_id":created["result"]["branch_id"],"operation_id":new_id("op")
        })).unwrap();
        file
    }

    fn directory_fixture(shared: &Shared) -> (String, String, String) {
        let identity = &shared.identities["alice"];
        let repositories = shared
            .backend
            .call("repositories", identity, json!({}))
            .unwrap();
        let repository = repositories["repositories"][0]["repository_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let created = shared
            .backend
            .call(
                "file_manager_directory_branches",
                identity,
                json!({
                    "slot_id":"P01","path":"资料","name":"draft","operation_id":new_id("op")
                }),
            )
            .unwrap();
        let initial = shared
            .backend
            .call(
                "default_branch",
                identity,
                json!({"repository_id":repository}),
            )
            .unwrap();
        (
            repository,
            initial["directory_branch_id"].as_str().unwrap().to_owned(),
            created["result"]["directory_branch_id"]
                .as_str()
                .unwrap()
                .to_owned(),
        )
    }

    fn select_directory(shared: &Shared, repository: &str, directory: &str) {
        shared.backend.call("update_default_branch", &shared.identities["alice"], json!({
            "repository_id":repository,"directory_branch_id":directory,"operation_id":new_id("op")
        })).unwrap();
    }

    #[test]
    fn directory_default_switch_fences_open_and_already_published_writers() {
        let (_temp, shared, backend, branch) = fixture();
        let (repository, main, draft) = directory_fixture(&shared);
        let mut one = session(shared.clone(), &branch);
        let (opened, file) = open_file(&mut one, true);
        file.write_all_at(b"new", 0).unwrap();
        select_directory(&shared, &repository, &draft);
        let error = one.dirty(opened.handle).unwrap_err();
        assert_eq!(error.code, "DIRECTORY_BRANCH_CONFLICT");
        assert_eq!(error.status, 409);
        let writer = match &one.handles[&opened.handle] {
            Handle::Write(w) => w,
            _ => unreachable!(),
        };
        assert!(writer.source.is_file());
        assert!(writer.source.with_extension("work.json").is_file());
        assert_eq!(backend.uploads.load(Ordering::SeqCst), 0);
        assert_eq!(one.find("report.txt").unwrap().unwrap().size, 3);
        one.close(opened.handle, true).unwrap();

        let (opened, file) = open_file(&mut one, true);
        file.write_all_at(b"own", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        one.flush(opened.handle).unwrap();
        // Return through another root selection, then to this exact content.
        // Equal file tuples must still retain the old handle's destination.
        select_directory(&shared, &repository, &main);
        select_directory(&shared, &repository, &draft);
        file.write_all_at(b"bad", 0).unwrap();
        assert_eq!(
            one.dirty(opened.handle).unwrap_err().code,
            "DIRECTORY_BRANCH_CONFLICT"
        );
        assert_eq!(backend.uploads.load(Ordering::SeqCst), 1);
        let writer = match &one.handles[&opened.handle] {
            Handle::Write(w) => w,
            _ => unreachable!(),
        };
        assert!(writer.dirty && writer.source.is_file());
        one.close(opened.handle, true).unwrap();
        let current = one.find("report.txt").unwrap().unwrap();
        let content = backend.content(&one.identity, &current.version_id).unwrap();
        let mut bytes = [0; 3];
        content.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"own");
    }

    #[test]
    fn namespace_move_transfers_only_selected_directory_views_and_keeps_read_handle() {
        let (_temp, shared, backend, branch) = fixture();
        let mut client = session(shared.clone(), &branch);
        for (path, bytes) in [
            ("a/report.txt", b"source".as_slice()),
            ("b/stay.txt", b"sibling".as_slice()),
        ] {
            let (opened, fd) = client
                .open(path.into(), true, true, false, false, false)
                .unwrap();
            let opened: OpenResult = serde_json::from_value(opened).unwrap();
            File::from(fd.unwrap()).write_all_at(bytes, 0).unwrap();
            client.dirty(opened.handle).unwrap();
            client.close(opened.handle, false).unwrap();
        }
        let identity = &shared.identities["alice"];
        let repo = shared
            .backend
            .call("repositories", identity, json!({}))
            .unwrap()["repositories"][0]["repository_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let register = |path: &str, name: &str| {
            shared.backend.call("file_manager_directory_branches",identity,json!({
            "slot_id":"P01","path":format!("资料/{path}"),"name":name,"operation_id":new_id("op")
        })).unwrap()["result"]["directory_branch_id"].as_str().unwrap().to_owned()
        };
        let a_m = register("a", "m");
        let a_n = register("a", "n");
        let b_l = register("b", "l");
        let b_p = register("b", "p");
        for (path, name) in [("a", "m"), ("b", "l")] {
            shared.backend.call("file_manager_directory_select",identity,json!({
                "slot_id":"P01","path":format!("资料/{path}"),"name":name,"operation_id":new_id("op")
            })).unwrap();
        }
        let scope = |id: &str| {
            shared
                .backend
                .call(
                    "directory_branch_scopes",
                    identity,
                    json!({"repository_id":repo}),
                )
                .unwrap()["directory_branches"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["directory_branch_id"] == id)
                .unwrap()
                .clone()
        };
        let m0 = scope(&a_m);
        let n0 = scope(&a_n);
        let l0 = scope(&b_l);
        let p0 = scope(&b_p);
        let source = client.find("a/report.txt").unwrap().unwrap();
        let (opened, fd) = client
            .open("a/report.txt".into(), false, false, false, false, false)
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        let reader = File::from(fd.unwrap());
        client
            .dispatch(Request::Namespace {
                kind: "rename".into(),
                path: "a/report.txt".into(),
                destination: Some("b/report.txt".into()),
                replace: false,
            })
            .unwrap();
        assert!(client.find("a/report.txt").unwrap().is_none());
        let moved = client.find("b/report.txt").unwrap().unwrap();
        assert_eq!(moved.file_id, source.file_id);
        assert_eq!(moved.version_id, source.version_id);
        let m1 = scope(&a_m);
        let l1 = scope(&b_l);
        assert_ne!(m1["head_tree_id"], m0["head_tree_id"]);
        assert_ne!(l1["head_tree_id"], l0["head_tree_id"]);
        assert_eq!(scope(&a_n)["head_tree_id"], n0["head_tree_id"]);
        assert_eq!(scope(&b_p)["head_tree_id"], p0["head_tree_id"]);
        let mut bytes = [0; 6];
        reader.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"source");
        client.close(opened.handle, false).unwrap();
        let (opened, fd) = client
            .open("b/report.txt".into(), true, false, false, false, false)
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        File::from(fd.unwrap()).write_all_at(b"edited", 0).unwrap();
        client.dirty(opened.handle).unwrap();
        client.close(opened.handle, false).unwrap();
        assert_eq!(
            scope(&a_m)["head_tree_id"],
            m1["head_tree_id"],
            "source stopped tracking the moved file"
        );
        assert_ne!(scope(&b_l)["head_tree_id"], l1["head_tree_id"]);
        assert_eq!(scope(&a_n)["head_tree_id"], n0["head_tree_id"]);
        assert_eq!(scope(&b_p)["head_tree_id"], p0["head_tree_id"]);
        let previous = backend.content(identity, &source.version_id).unwrap();
        previous.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"source");
    }

    #[test]
    fn newly_created_ancestor_selection_fences_an_existing_create_handle() {
        let (_temp, shared, _backend, branch) = fixture();
        let mut original = session(shared.clone(), &branch);
        let (opened, file) = original
            .open("future/new.txt".into(), true, true, false, false, false)
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        let file = File::from(file.unwrap());
        let mut creator = session(shared.clone(), &branch);
        let (other, other_fd) = creator
            .open("future/other.txt".into(), true, true, false, false, false)
            .unwrap();
        let other: OpenResult = serde_json::from_value(other).unwrap();
        File::from(other_fd.unwrap())
            .write_all_at(b"other", 0)
            .unwrap();
        creator.dirty(other.handle).unwrap();
        creator.close(other.handle, false).unwrap();
        shared.backend.call("file_manager_directory_branches", &shared.identities["alice"], json!({
            "slot_id":"P01","path":"资料/future","name":"alternate","operation_id":new_id("op")
        })).unwrap();
        shared.backend.call("file_manager_directory_select", &shared.identities["alice"], json!({
            "slot_id":"P01","path":"资料/future","name":"alternate","operation_id":new_id("op")
        })).unwrap();
        file.write_all_at(b"keep", 0).unwrap();
        assert_eq!(
            original.dirty(opened.handle).unwrap_err().code,
            "DIRECTORY_BRANCH_CONFLICT"
        );
        let writer = match &original.handles[&opened.handle] {
            Handle::Write(w) => w,
            _ => unreachable!(),
        };
        assert!(writer.source.is_file() && writer.dirty);
        assert!(original.find("future/new.txt").unwrap().is_none());
        original.close(opened.handle, true).unwrap();
    }

    #[test]
    fn nested_directory_switch_does_not_fence_an_unrelated_open_file() {
        let (_temp, shared, backend, branch) = fixture();
        let mut creator = session(shared.clone(), &branch);
        let (opened, file) = creator
            .open("nested/other.txt".into(), true, true, false, false, false)
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        File::from(file.unwrap()).write_all_at(b"other", 0).unwrap();
        creator.dirty(opened.handle).unwrap();
        creator.close(opened.handle, false).unwrap();
        let (_repository, _main, _draft) = directory_fixture(&shared);
        shared.backend.call("file_manager_directory_branches", &shared.identities["alice"], json!({
            "slot_id":"P01","path":"资料/nested","name":"alternate","operation_id":new_id("op")
        })).unwrap();
        let mut one = session(shared.clone(), &branch);
        let (opened, file) = open_file(&mut one, true);
        shared.backend.call("file_manager_directory_select", &shared.identities["alice"], json!({
            "slot_id":"P01","path":"资料/nested","name":"alternate","operation_id":new_id("op")
        })).unwrap();
        file.write_all_at(b"new", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        one.close(opened.handle, false).unwrap();
        let current = one.find("report.txt").unwrap().unwrap();
        let content = backend.content(&one.identity, &current.version_id).unwrap();
        let mut bytes = [0; 3];
        content.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"new");
    }

    #[test]
    fn unix_rpc_default_switch_preserves_read_fds_and_targets_new_writes() {
        let (_temp, shared, backend, branch) = fixture();
        let (repository, main, draft) = directory_fixture(&shared);
        select_directory(&shared, &repository, &draft);
        let mut initial = session(shared.clone(), &branch);
        let (opened, file) = open_file(&mut initial, true);
        file.write_all_at(b"new", 0).unwrap();
        initial.dirty(opened.handle).unwrap();
        initial.close(opened.handle, false).unwrap();

        let (client, server) = std::os::unix::net::UnixStream::pair().unwrap();
        let server_shared = shared.clone();
        let server = std::thread::spawn(move || {
            run_connection(FrameStream::new(server).unwrap(), server_shared).unwrap()
        });
        let mut wire = FrameStream::new(client).unwrap();
        let rpc = |wire: &mut FrameStream, request: Request| {
            wire.send(&request, None).unwrap();
            wire.receive::<Response>().unwrap()
        };
        let (hello, fd) = rpc(
            &mut wire,
            Request::Hello {
                protocol: PROTOCOL.into(),
                username: "alice".into(),
                repository_id: repository.clone(),
            },
        );
        assert!(matches!(hello, Response::Ok { .. }) && fd.is_none());
        let open = || Request::Open {
            path: "report.txt".into(),
            writable: false,
            create: false,
            truncate: false,
            exclusive: false,
            write_through: false,
        };
        let (Response::Ok { result }, pinned) = rpc(&mut wire, open()) else {
            panic!("read open failed")
        };
        let pinned_open: OpenResult = serde_json::from_value(result).unwrap();
        let pinned = File::from(pinned.unwrap());
        let mut bytes = [0; 3];
        pinned.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"new");
        select_directory(&shared, &repository, &main);
        let (Response::Ok { result }, visible) = rpc(&mut wire, open()) else {
            panic!("read after switch failed")
        };
        let visible_open: OpenResult = serde_json::from_value(result).unwrap();
        File::from(visible.unwrap())
            .read_exact_at(&mut bytes, 0)
            .unwrap();
        assert_eq!(&bytes, b"old");
        pinned.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"new", "existing immutable read FD stays pinned");
        let (Response::Ok { result }, writable_fd) = rpc(
            &mut wire,
            Request::Open {
                path: "report.txt".into(),
                writable: true,
                create: false,
                truncate: false,
                exclusive: false,
                write_through: false,
            },
        ) else {
            panic!("write after switch failed")
        };
        let writable: OpenResult = serde_json::from_value(result).unwrap();
        let writable_fd = File::from(writable_fd.unwrap());
        writable_fd.write_all_at(b"smb", 0).unwrap();
        assert!(matches!(
            rpc(
                &mut wire,
                Request::Dirty {
                    handle: writable.handle
                }
            )
            .0,
            Response::Ok { .. }
        ));
        assert!(matches!(
            rpc(
                &mut wire,
                Request::Close {
                    handle: writable.handle
                }
            )
            .0,
            Response::Ok { .. }
        ));
        for handle in [pinned_open.handle, visible_open.handle] {
            assert!(matches!(
                rpc(&mut wire, Request::Close { handle }).0,
                Response::Ok { .. }
            ));
        }
        drop(wire);
        server.join().unwrap();
        let observer = session(shared.clone(), &branch);
        let entry = observer.find("report.txt").unwrap().unwrap();
        let content = backend
            .content(&observer.identity, &entry.version_id)
            .unwrap();
        content.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"smb");
        select_directory(&shared, &repository, &draft);
        let entry = observer.find("report.txt").unwrap().unwrap();
        backend
            .content(&observer.identity, &entry.version_id)
            .unwrap()
            .file
            .read_exact_at(&mut bytes, 0)
            .unwrap();
        assert_eq!(
            &bytes, b"new",
            "saving a different combination preserves the original frozen view"
        );
    }

    #[test]
    fn unwritten_branch_reads_parent_and_materializes_only_on_dirty_save() {
        let (_temp, shared, backend, branch) = fixture();
        let file = select_pending_file(&shared, &branch);
        let history = || {
            shared
                .backend
                .call(
                    "file_manager_file_history",
                    &shared.identities["alice"],
                    json!({"file_id":file}),
                )
                .unwrap()
        };
        let before = history();
        assert_eq!(before["versions"].as_array().unwrap().len(), 1);
        let mut session = session(shared.clone(), &branch);
        assert_eq!(before["inherited"], true);
        assert!(before["own_version_id"].is_null());
        let (parent_read, parent_content) = open_file(&mut session, false);
        let mut parent_bytes = [0u8; 3];
        parent_content.read_exact_at(&mut parent_bytes, 0).unwrap();
        assert_eq!(&parent_bytes, b"old");
        let (opened, staged) = open_file(&mut session, true);
        assert_eq!(opened.size, 3);
        assert_eq!(
            opened.version_id.as_deref(),
            before["versions"][0]["version_id"].as_str()
        );
        assert_eq!(staged.metadata().unwrap().len(), 3);
        let mut copied = [0u8; 3];
        staged.read_exact_at(&mut copied, 0).unwrap();
        assert_eq!(&copied, b"old");
        session.close(opened.handle, false).unwrap();
        assert_eq!(history(), before);
        assert_eq!(backend.uploads.load(Ordering::SeqCst), 0);

        let (opened, staged) = open_file(&mut session, true);
        staged.write_all_at(b"unpublished", 0).unwrap();
        session.dirty(opened.handle).unwrap();
        session.close(opened.handle, true).unwrap();
        assert_eq!(
            history(),
            before,
            "aborting an actual write retains the inherited branch record"
        );

        let (opened, staged) = open_file(&mut session, true);
        staged.write_all_at(b"X", 0).unwrap();
        session.dirty(opened.handle).unwrap();
        session.close(opened.handle, false).unwrap();
        let after = history();
        assert_eq!(after["versions"].as_array().unwrap().len(), 2);
        let current = after["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|version| version["current"] == true)
            .unwrap();
        assert_eq!(
            current["content_hash"],
            blake3::hash(b"Xld").to_hex().as_str()
        );
        assert_eq!(
            current["source_version_ids"],
            json!([before["versions"][0]["version_id"]])
        );
        let (read, mut content) = open_file(&mut session, false);
        let mut bytes = Vec::new();
        content.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"Xld");
        assert_eq!(after["inherited"], false);
        assert_eq!(after["materialized"], true);
        assert_eq!(after["own_version_id"], current["version_id"]);
        parent_content.read_exact_at(&mut parent_bytes, 0).unwrap();
        assert_eq!(
            &parent_bytes, b"old",
            "the inherited read handle keeps the parent version"
        );
        session.close(parent_read.handle, false).unwrap();
        session.close(read.handle, false).unwrap();
    }

    #[test]
    fn explicit_truncation_materializes_an_empty_pending_branch() {
        let (_temp, shared, _backend, branch) = fixture();
        let file = select_pending_file(&shared, &branch);
        let mut session = session(shared.clone(), &branch);
        let (opened, descriptor) = session
            .open("report.txt".into(), true, false, true, false, false)
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        drop(descriptor);
        session.close(opened.handle, false).unwrap();
        let history = shared
            .backend
            .call(
                "file_manager_file_history",
                &shared.identities["alice"],
                json!({"file_id":file}),
            )
            .unwrap();
        assert_eq!(history["versions"].as_array().unwrap().len(), 2);
        let current = history["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|version| version["current"] == true)
            .unwrap();
        assert_eq!(current["size"], "0");
        assert_eq!(current["content_hash"], blake3::hash(b"").to_hex().as_str());
        assert!(
            history["branches"]
                .as_array()
                .unwrap()
                .iter()
                .find(|branch| branch["current"] == true)
                .unwrap()["materialized"]
                .as_bool()
                .unwrap()
        );
    }

    #[test]
    fn actual_core_flush_publishes_full_file_and_old_reader_keeps_its_version() {
        let (_temp, shared, backend, branch) = fixture();
        let mut one = session(shared.clone(), &branch);
        let (read, old) = open_file(&mut one, false);
        let (write, file) = open_file(&mut one, true);
        file.write_all_at(b"new", 0).unwrap();
        one.dirty(write.handle).unwrap();
        let mut bytes = [0u8; 3];
        old.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"old");
        let saved = one.flush(write.handle).unwrap();
        assert_eq!(saved["commit_state"], "PUBLISHED");
        old.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"old");
        let new_entry = one
            .entries()
            .unwrap()
            .into_iter()
            .find(|entry| entry.path == "report.txt")
            .unwrap();
        let content = backend
            .content(&one.identity, &new_entry.version_id)
            .unwrap();
        content.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"new");
        one.close(write.handle, false).unwrap();
        one.close(read.handle, false).unwrap();
    }

    #[test]
    fn lost_commit_reply_reconciles_same_operation_without_restaging_or_second_version() {
        let (_temp, shared, backend, branch) = fixture();
        let mut one = session(shared, &branch);
        let (opened, file) = open_file(&mut one, true);
        file.write_all_at(b"new", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        backend.fail.store(true, Ordering::SeqCst);
        assert_eq!(one.flush(opened.handle).unwrap_err().code, "RESPONSE_LOST");
        let uploads = backend.uploads.load(Ordering::SeqCst);
        let saved = one.flush(opened.handle).unwrap();
        assert_eq!(saved["commit_state"], "PUBLISHED");
        assert_eq!(backend.uploads.load(Ordering::SeqCst), uploads);
        let entry = one
            .entries()
            .unwrap()
            .into_iter()
            .find(|entry| entry.path == "report.txt")
            .unwrap();
        let history = backend
            .call("history", &one.identity, json!({"file_id":entry.file_id}))
            .unwrap();
        assert_eq!(history["versions"].as_array().unwrap().len(), 2);
        one.close(opened.handle, false).unwrap();
    }

    #[test]
    fn cross_session_writer_is_denied_and_last_close_commits() {
        let (_temp, shared, _backend, branch) = fixture();
        let mut one = session(shared.clone(), &branch);
        let mut two = session(shared, &branch);
        let (opened, file) = open_file(&mut one, true);
        let error = two
            .open("report.txt".into(), true, false, false, false, false)
            .unwrap_err();
        assert_eq!(error.code, "FILE_WRITE_BUSY");
        file.write_all_at(b"end", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        one.close(opened.handle, false).unwrap();
        let (next, _) = open_file(&mut two, true);
        two.close(next.handle, false).unwrap();
    }

    #[test]
    fn different_files_publish_concurrently_and_reopen_workspace_after_unrelated_save() {
        let (_temp, shared, _backend, branch) = fixture();
        let mut one = session(shared.clone(), &branch);
        let mut two = session(shared, &branch);
        let (first, first_file) = open_file(&mut one, true);
        let (result, fd) = two
            .open("other.txt".into(), true, true, false, false, false)
            .unwrap();
        let second: OpenResult = serde_json::from_value(result).unwrap();
        let second_file = File::from(fd.unwrap());
        first_file.write_all_at(b"one", 0).unwrap();
        one.dirty(first.handle).unwrap();
        second_file.write_all_at(b"two", 0).unwrap();
        two.dirty(second.handle).unwrap();
        one.flush(first.handle).unwrap();
        two.flush(second.handle).unwrap();
        first_file.write_all_at(b"new", 0).unwrap();
        one.dirty(first.handle).unwrap();
        one.flush(first.handle).unwrap();
        let entries = one.entries().unwrap();
        assert_eq!(entries.iter().filter(|e| e.kind == "file").count(), 2);
        let other = entries.iter().find(|e| e.path == "other.txt").unwrap();
        let content = one
            .shared
            .backend
            .content(&one.identity, &other.version_id)
            .unwrap();
        let mut bytes = [0u8; 3];
        content.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"two");
        one.close(first.handle, false).unwrap();
        two.close(second.handle, false).unwrap();
    }
    #[test]
    fn namespace_locks_cover_only_affected_paths_and_case_only_replace_preserves_entry() {
        let (_temp, shared, _backend, branch) = fixture();
        let mut one = session(shared.clone(), &branch);
        let mut two = session(shared, &branch);
        let before = one.find("report.txt").unwrap().unwrap();
        two.dispatch(Request::Namespace {
            kind: "rename".into(),
            path: "report.txt".into(),
            destination: Some("REPORT.txt".into()),
            replace: true,
        })
        .unwrap();
        let after = two.find("REPORT.txt").unwrap().unwrap();
        assert_eq!(after.entry_id, before.entry_id);
        assert_eq!(after.version_id, before.version_id);
        assert_eq!(after.path, "REPORT.txt");
        let (opened, fd) = one
            .open("dir/file.txt".into(), true, true, false, false, false)
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        let file = File::from(fd.unwrap());
        assert_eq!(
            two.dispatch(Request::Namespace {
                kind: "rename".into(),
                path: "dir".into(),
                destination: Some("moved".into()),
                replace: false
            })
            .unwrap_err()
            .code,
            "FILE_WRITE_BUSY"
        );
        two.dispatch(Request::Namespace {
            kind: "mkdir".into(),
            path: "unrelated".into(),
            destination: None,
            replace: false,
        })
        .unwrap();
        file.write_all_at(b"bytes", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        one.close(opened.handle, false).unwrap();
        assert!(two.find("unrelated").unwrap().is_some());
        assert!(two.find("dir/file.txt").unwrap().is_some());
        assert_eq!(
            two.dispatch(Request::Namespace {
                kind: "rename".into(),
                path: "REPORT.txt".into(),
                destination: Some("unrelated".into()),
                replace: true
            })
            .unwrap_err()
            .code,
            "INVALID_REQUEST"
        );
        two.dispatch(Request::Namespace {
            kind: "mkdir".into(),
            path: "after-failure".into(),
            destination: None,
            replace: false,
        })
        .unwrap();
    }

    #[test]
    fn listing_uses_portable_lookup_keys_and_rejects_files_or_missing_directories() {
        let (_temp, shared, _backend, branch) = fixture();
        let mut session = session(shared, &branch);
        for path in ["Straße", "STRASSE/子目录", "Straße-other"] {
            session
                .dispatch(Request::Namespace {
                    kind: "mkdir".into(),
                    path: path.into(),
                    destination: None,
                    replace: false,
                })
                .unwrap();
        }
        let (opened, fd) = session
            .open(
                "STRASSE/子目录/内容.txt".into(),
                true,
                true,
                false,
                false,
                false,
            )
            .unwrap();
        let opened: OpenResult = serde_json::from_value(opened).unwrap();
        let file = File::from(fd.unwrap());
        file.write_all_at(b"content", 0).unwrap();
        session.dirty(opened.handle).unwrap();
        session.close(opened.handle, false).unwrap();
        let (listed, _) = session
            .dispatch(Request::List {
                path: "straße".into(),
            })
            .unwrap();
        let entries = listed["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["name"], "子目录");
        let (listed, _) = session
            .dispatch(Request::List {
                path: "strasse/子目录".into(),
            })
            .unwrap();
        let entries = listed["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["name"], "内容.txt");
        assert_eq!(
            session
                .dispatch(Request::List {
                    path: "REPORT.TXT".into()
                })
                .unwrap_err()
                .code,
            "NOT_DIRECTORY"
        );
        assert_eq!(
            session
                .dispatch(Request::List {
                    path: "absent".into()
                })
                .unwrap_err()
                .status,
            404
        );
    }

    #[test]
    fn changed_bytes_after_lost_publication_reply_are_retained_for_a_second_save() {
        let (_temp, shared, backend, branch) = fixture();
        let mut one = session(shared, &branch);
        let (opened, file) = open_file(&mut one, true);
        file.write_all_at(b"new", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        backend.fail.store(true, Ordering::SeqCst);
        assert_eq!(one.flush(opened.handle).unwrap_err().code, "RESPONSE_LOST");
        file.write_all_at(b"end", 0).unwrap();
        assert_eq!(
            one.flush(opened.handle).unwrap_err().code,
            "WORKSPACE_REBASE_REQUIRED"
        );
        one.dirty(opened.handle).unwrap();
        one.flush(opened.handle).unwrap();
        let current = one.find("report.txt").unwrap().unwrap();
        let content = backend.content(&one.identity, &current.version_id).unwrap();
        let mut bytes = [0u8; 3];
        content.file.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"end");
        one.close(opened.handle, false).unwrap();
    }

    #[test]
    fn external_same_file_change_rejects_rebase_and_preserves_work_bytes() {
        let (temp, shared, backend, branch) = fixture();
        let mut one = session(shared, &branch);
        let (opened, file) = open_file(&mut one, true);
        file.write_all_at(b"one", 0).unwrap();
        one.dirty(opened.handle).unwrap();
        one.flush(opened.handle).unwrap();
        let workspace: Workspace = serde_json::from_value(
            backend
                .call(
                    "create_workspace",
                    &one.identity,
                    json!({"branch_id":branch}),
                )
                .unwrap(),
        )
        .unwrap();
        let source = temp.path().join("external.txt");
        fs::write(&source, b"api").unwrap();
        backend
            .upload(
                &one.identity,
                &workspace.workspace_id,
                &opened.entry_id,
                "report.txt",
                &source,
            )
            .unwrap();
        backend.call("commit_workspace_internal",&one.identity,json!({"workspace_id":workspace.workspace_id,"operation_id":new_id("op"),"expected_head":workspace.base_head,"fencing_token":workspace.fencing_token.to_string(),"message":"external app"})).unwrap();
        file.write_all_at(b"end", 0).unwrap();
        assert_eq!(
            one.dirty(opened.handle).unwrap_err().code,
            "FILE_VERSION_CONFLICT"
        );
        assert_eq!(
            one.close(opened.handle, false).unwrap_err().code,
            "WORKSPACE_REBASE_REQUIRED"
        );
        let pending = match &one.handles[&opened.handle] {
            Handle::Write(writer) => writer.source.clone(),
            _ => panic!("write handle missing"),
        };
        drop(one);
        assert_eq!(fs::read(pending).unwrap(), b"end");
    }

    #[test]
    fn already_open_reader_checks_current_account_before_each_read_request() {
        let (_temp, shared, backend, branch) = fixture();
        let mut one = session(shared, &branch);
        let (opened, _file) = open_file(&mut one, false);
        one.dispatch(Request::AuthorizeRead {
            handle: opened.handle,
        })
        .unwrap();
        backend.revoked.store(true, Ordering::SeqCst);
        assert_eq!(
            one.dispatch(Request::AuthorizeRead {
                handle: opened.handle
            })
            .unwrap_err()
            .code,
            "FORBIDDEN"
        );
        assert!(one.handles.contains_key(&opened.handle));
        one.close(opened.handle, false).unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancellation_closes_blocked_rpc_sessions_and_removes_its_socket() {
        let (temp, shared, _backend, _branch) = fixture();
        let parent = temp.path().join("rpc");
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = parent.join("smb.sock");
        let server_socket = socket.clone();
        let identities = shared.identities.clone();
        let backend = shared.backend.clone();
        let server = tokio::spawn(async move {
            serve(
                &server_socket,
                xcs_smb::protocol::effective_uid(),
                identities,
                backend,
            )
            .await
        });
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let stream = std::os::unix::net::UnixStream::connect(&socket).unwrap();
        server.abort();
        let _ = server.await;
        assert!(!socket.exists());
        drop(stream);
    }
    fn immutable_fixture_payloads(root: &Path) -> Vec<PathBuf> {
        fn visit(root: &Path, found: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(root).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path, found);
                } else if path
                    .components()
                    .any(|part| part.as_os_str() == "file-containers")
                    && fs::read(&path).unwrap() == b"old"
                {
                    found.push(path);
                }
            }
        }
        let mut found = Vec::new();
        visit(root, &mut found);
        found.sort();
        found
    }

    #[test]
    fn raw_fd_and_writer_copy_reject_corruption_and_use_verified_fallback() {
        for writable in [false, true] {
            let (temp, shared, _backend, branch) = fixture();
            let payloads = immutable_fixture_payloads(temp.path());
            assert_eq!(payloads.len(), 3);
            // Corrupt the primary specifically, leaving the backup replicas valid.
            let primary = payloads
                .iter()
                .find(|path| path.starts_with(temp.path().join("p")))
                .unwrap();
            fs::set_permissions(primary, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(primary, b"bad").unwrap();
            let mut active = session(shared.clone(), &branch);
            let (opened, file) = open_file(&mut active, writable);
            let mut bytes = [0u8; 3];
            file.read_exact_at(&mut bytes, 0).unwrap();
            assert_eq!(
                &bytes, b"old",
                "raw FD or writer copy exposed corrupted bytes"
            );
            active.close(opened.handle, true).unwrap();
            drop(active);
            for path in payloads {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(path, b"bad").unwrap();
            }
            let mut denied = session(shared, &branch);
            let result = denied.open("report.txt".into(), writable, false, false, false, false);
            assert!(matches!(result, Err(ref error) if error.code == "CONTENT_UNAVAILABLE"));
            assert!(denied.handles.is_empty(), "corrupt descriptor was admitted");
        }
    }
}
