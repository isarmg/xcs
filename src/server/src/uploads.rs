//! Durable full-upload staging. Interrupted uploads remain attributable and queryable.
use crate::ApiError;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UploadRecord {
    pub workspace_id: String,
    pub item_id: String,
    pub principal: String,
    pub path: String,
    #[serde(with = "storage_contract::u64_string")]
    pub generation: u64,
    #[serde(with = "storage_contract::u64_string")]
    pub logical_size: u64,
    pub content_hash: Option<String>,
    #[serde(with = "storage_contract::u64_string")]
    pub persisted_bytes: u64,
    pub persisted_hash: String,
    pub state: String,
    pub data_file: String,
}
pub(crate) fn key(workspace: &str, item: &str) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(workspace.as_bytes());
    hash.update(&[0]);
    hash.update(item.as_bytes());
    hash.finalize().to_hex().to_string()
}
pub(crate) fn load(
    root: &Path,
    workspace: &str,
    item: &str,
) -> Result<Option<UploadRecord>, ApiError> {
    let path = root.join(format!("{}.json", key(workspace, item)));
    match fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            // The spool is private, but a damaged file must still not force an
            // unbounded allocation before the metadata size is checked.
            file.take(16385)
                .read_to_end(&mut bytes)
                .map_err(ApiError::io)?;
            if bytes.len() > 16384 {
                return Err(ApiError::new(
                    500,
                    "UPLOAD_STATE_INVALID",
                    "invalid upload state",
                ));
            }
            let record = storage_contract::decode_strict::<UploadRecord>(&bytes)
                .map_err(|_| ApiError::new(500, "UPLOAD_STATE_INVALID", "invalid upload state"))?;
            if record.workspace_id != workspace
                || record.item_id != item
                || record.persisted_bytes > record.logical_size
                || record.data_file.contains('/')
                || record.data_file.contains('\\')
                || record.data_file.starts_with('.')
                || record.data_file.is_empty()
                || record.generation == 0
                || record.content_hash.as_ref().is_none_or(|hash| {
                    hash.len() != 64
                        || !hash
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
                || record.persisted_hash.len() != 64
                || !record
                    .persisted_hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(ApiError::new(
                    500,
                    "UPLOAD_STATE_INVALID",
                    "invalid upload state",
                ));
            }
            Ok(Some(record))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ApiError::io(error)),
    }
}
/// An unreferenced request file is disposable. Once a durable record is renamed
/// into place it owns these bytes, even if the following directory sync fails.
/// Blocking persistence jobs share this guard, so cancellation cannot delete a
/// file just before the job publishes its record.
pub(crate) struct StagedFile {
    path: PathBuf,
    referenced: AtomicBool,
    superseded: Option<PathBuf>,
    superseded_removed: AtomicBool,
}
impl StagedFile {
    pub fn create(
        path: PathBuf,
        superseded: Option<PathBuf>,
    ) -> Result<(fs::File, std::sync::Arc<Self>), ApiError> {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path).map_err(ApiError::io)?;
        Ok((
            file,
            std::sync::Arc::new(Self {
                path,
                referenced: AtomicBool::new(false),
                superseded,
                superseded_removed: AtomicBool::new(false),
            }),
        ))
    }
}
impl Drop for StagedFile {
    fn drop(&mut self) {
        if !self.referenced.load(Ordering::Acquire)
            && let Err(error) = fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "unreferenced upload cleanup failed");
        }
    }
}
pub(crate) fn persist_staged(
    root: &Path,
    record: &UploadRecord,
    staged: &StagedFile,
) -> Result<(), ApiError> {
    persist_staged_with_sync(root, record, staged, || sync_directory(root))
}
fn sync_directory(root: &Path) -> Result<(), ApiError> {
    fs::File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(ApiError::io)
}
fn persist_staged_with_sync(
    root: &Path,
    record: &UploadRecord,
    staged: &StagedFile,
    sync: impl FnOnce() -> Result<(), ApiError>,
) -> Result<(), ApiError> {
    persist_inner(
        root,
        record,
        || staged.referenced.store(true, Ordering::Release),
        sync,
    )?;
    // Only after the new data and metadata are durable may a same-generation
    // predecessor lose its last reference. Older generations are never listed.
    if let Some(previous) = &staged.superseded
        && !staged.superseded_removed.swap(true, Ordering::AcqRel)
        && let Err(error) = fs::remove_file(previous)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        staged.superseded_removed.store(false, Ordering::Release);
        tracing::warn!(%error, "superseded upload cleanup failed");
    }
    Ok(())
}
#[cfg(test)]
pub(crate) fn persist(root: &Path, record: &UploadRecord) -> Result<(), ApiError> {
    persist_inner(root, record, || {}, || sync_directory(root))
}
fn persist_inner(
    root: &Path,
    record: &UploadRecord,
    published: impl FnOnce(),
    sync: impl FnOnce() -> Result<(), ApiError>,
) -> Result<(), ApiError> {
    let name = key(&record.workspace_id, &record.item_id);
    let target = root.join(format!("{name}.json"));
    let temporary = root.join(format!(
        "{name}.json.tmp-{}",
        authentication::random_token().map_err(ApiError::invalid)?
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(ApiError::io)?;
    let result = (|| {
        file.write_all(&serde_json::to_vec(record).map_err(ApiError::invalid)?)
            .map_err(ApiError::io)?;
        file.sync_all().map_err(ApiError::io)?;
        fs::rename(&temporary, target).map_err(ApiError::io)?;
        published();
        sync()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub(crate) fn status(record: &UploadRecord) -> serde_json::Value {
    serde_json::json!({"workspace_id":record.workspace_id,"item_id":record.item_id,"upload_generation":record.generation.to_string(),"logical_size":record.logical_size.to_string(),"content_hash":record.content_hash,"state":if record.state=="complete"{"COMPLETE"}else{"RECEIVING"},"durable_ranges":if record.persisted_bytes>0{vec![serde_json::json!({"offset":"0","length":record.persisted_bytes.to_string(),"content_hash":record.persisted_hash})]}else{vec![]},"sealed_manifest_hash":null})
}
pub(crate) fn retain_generation(root: &Path, record: &UploadRecord) -> Result<(), ApiError> {
    let path = root.join(format!(
        "retained-{}-{}-{}.json",
        key(&record.workspace_id, &record.item_id),
        record.generation,
        authentication::random_token().map_err(ApiError::invalid)?
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(ApiError::io)?;
    file.write_all(&serde_json::to_vec(record).map_err(ApiError::invalid)?)
        .map_err(ApiError::io)?;
    file.sync_all().map_err(ApiError::io)?;
    fs::File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(ApiError::io)
}
pub(crate) struct ExistingPrefix {
    file: fs::File,
    remaining: u64,
}
impl ExistingPrefix {
    pub fn open(root: &Path, record: &UploadRecord) -> Result<Option<Self>, ApiError> {
        if record.persisted_bytes == 0 {
            return Ok(None);
        }
        let file = fs::File::open(root.join(&record.data_file)).map_err(ApiError::io)?;
        if file.metadata().map_err(ApiError::io)?.len() < record.persisted_bytes {
            return Err(ApiError::new(
                500,
                "UPLOAD_STATE_INVALID",
                "persisted upload data is missing",
            ));
        }
        Ok(Some(Self {
            file,
            remaining: record.persisted_bytes,
        }))
    }
    pub fn check(&mut self, bytes: &[u8]) -> Result<(), ApiError> {
        let length = bytes
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let mut expected = vec![0u8; length];
        self.file.read_exact(&mut expected).map_err(ApiError::io)?;
        if expected != bytes[..length] {
            return Err(ApiError::conflict(
                "UPLOAD_RANGE_CONFLICT",
                "same upload generation contains different bytes; retain this generation and explicitly start a newer one",
            ));
        }
        self.remaining -= length as u64;
        Ok(())
    }
    pub fn complete(&self) -> bool {
        self.remaining == 0
    }
}
pub(crate) struct Claim {
    pub claims: std::sync::Arc<std::sync::Mutex<std::collections::BTreeSet<String>>>,
    pub key: String,
}
impl Drop for Claim {
    fn drop(&mut self) {
        if let Ok(mut claims) = self.claims.lock() {
            claims.remove(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staging_guard_preserves_referenced_files_and_cleans_up_unpublished_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("unpublished.data");
        let (file, guard) = StagedFile::create(path.clone(), None).unwrap();
        let running_job = guard.clone();
        drop(guard);
        assert!(path.exists(), "an outstanding blocking job owns staging");
        drop(file);
        drop(running_job);
        assert!(!path.exists());
        let path = root.path().join("published.data");
        let (file, guard) = StagedFile::create(path.clone(), None).unwrap();
        guard.referenced.store(true, Ordering::Release);
        drop(file);
        drop(guard);
        assert!(path.exists());
        // Failed exclusive creation must never assume ownership of existing data.
        assert!(StagedFile::create(path.clone(), None).is_err());
        assert!(path.exists());
    }

    #[test]
    fn uncertain_directory_sync_preserves_both_old_and_new_data() {
        let root = tempfile::tempdir().unwrap();
        let previous = root.path().join("previous.data");
        fs::write(&previous, b"previous").unwrap();
        let path = root.path().join("new.data");
        let (mut file, guard) = StagedFile::create(path.clone(), Some(previous.clone())).unwrap();
        file.write_all(b"new bytes").unwrap();
        file.sync_all().unwrap();
        let record = UploadRecord {
            workspace_id: "ws".into(),
            item_id: "item".into(),
            principal: "alice".into(),
            path: "file".into(),
            generation: 1,
            logical_size: 9,
            content_hash: Some(blake3::hash(b"new bytes").to_hex().to_string()),
            persisted_bytes: 9,
            persisted_hash: blake3::hash(b"new bytes").to_hex().to_string(),
            state: "incomplete".into(),
            data_file: "new.data".into(),
        };
        let result = persist_staged_with_sync(root.path(), &record, &guard, || {
            Err(ApiError::io(std::io::Error::other(
                "injected directory sync failure",
            )))
        });
        assert!(result.is_err());
        assert_eq!(
            load(root.path(), "ws", "item").unwrap().unwrap().data_file,
            "new.data"
        );
        drop(file);
        drop(guard);
        assert_eq!(fs::read(path).unwrap(), b"new bytes");
        assert_eq!(fs::read(previous).unwrap(), b"previous");
        // The old file is intentionally unindexed uncertainty evidence. A later
        // request must not guess which record a real crash would have retained.
    }

    #[test]
    fn interrupted_upload_retains_and_checks_its_persisted_prefix() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data"), b"original partial").unwrap();
        let record = UploadRecord {
            workspace_id: "ws".into(),
            item_id: "item".into(),
            principal: "alice".into(),
            path: "file".into(),
            generation: 1,
            logical_size: 30,
            content_hash: Some("0".repeat(64)),
            persisted_bytes: 16,
            persisted_hash: blake3::hash(b"original partial").to_hex().to_string(),
            state: "incomplete".into(),
            data_file: "data".into(),
        };
        persist(dir.path(), &record).unwrap();
        let loaded = load(dir.path(), "ws", "item").unwrap().unwrap();
        assert_eq!(loaded.persisted_bytes, 16);
        let mut prefix = ExistingPrefix::open(dir.path(), &loaded).unwrap().unwrap();
        assert!(prefix.check(b"different partial").is_err());
        let mut prefix = ExistingPrefix::open(dir.path(), &loaded).unwrap().unwrap();
        prefix.check(b"original partial and more").unwrap();
        assert!(prefix.complete());
        assert_eq!(status(&loaded)["state"], "RECEIVING");
    }

    #[test]
    fn corrupt_or_oversized_progress_is_an_internal_state_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{}.json", key("ws", "item")));
        for bytes in [
            b"{broken".to_vec(),
            vec![b' '; 16_385],
            vec![b' '; 1024 * 1024],
        ] {
            fs::write(&path, bytes).unwrap();
            let error = load(dir.path(), "ws", "item").err().unwrap();
            assert_eq!(error.status, 500);
            assert_eq!(error.code, "UPLOAD_STATE_INVALID");
        }
    }

    #[test]
    fn failed_atomic_progress_publication_cleans_up_its_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let record = UploadRecord {
            workspace_id: "ws".into(),
            item_id: "item".into(),
            principal: "alice".into(),
            path: "file".into(),
            generation: 1,
            logical_size: 0,
            content_hash: Some(blake3::hash(b"").to_hex().to_string()),
            persisted_bytes: 0,
            persisted_hash: blake3::hash(b"").to_hex().to_string(),
            state: "incomplete".into(),
            data_file: "data".into(),
        };
        // A directory at the destination makes rename fail deterministically,
        // even when the test runs as root and chmod cannot deny writes.
        fs::create_dir(dir.path().join(format!("{}.json", key("ws", "item")))).unwrap();
        assert!(persist(dir.path(), &record).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
