//! Durable full-upload staging. Interrupted uploads remain attributable and queryable.
use crate::ApiError;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UploadRecord {
    pub workspace_id: String,
    pub item_id: String,
    pub principal: String,
    pub path: String,
    #[serde(with = "zxs_contract::u64_string")]
    pub generation: u64,
    #[serde(with = "zxs_contract::u64_string")]
    pub logical_size: u64,
    pub content_hash: Option<String>,
    #[serde(with = "zxs_contract::u64_string")]
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
            let record = zxs_contract::decode_strict::<UploadRecord>(&bytes)
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
pub(crate) fn persist(root: &Path, record: &UploadRecord) -> Result<(), ApiError> {
    let name = key(&record.workspace_id, &record.item_id);
    let target = root.join(format!("{name}.json"));
    let temporary = root.join(format!(
        "{name}.json.tmp-{}",
        crate::auth_primitives::random_token().map_err(ApiError::invalid)?
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
        fs::File::open(root)
            .and_then(|file| file.sync_all())
            .map_err(ApiError::io)
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
        crate::auth_primitives::random_token().map_err(ApiError::invalid)?
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
