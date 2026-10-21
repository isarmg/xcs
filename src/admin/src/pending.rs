//! Complete no-clobber publication of configuration and retry request files.
use crate::{Result, private_io};
use platform_io::fs_safety::PrivateDirectory;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) fn save_pending(root: &Path, operation_id: &str, body: &Value) -> Result<PathBuf> {
    storage_engine::validate_id(operation_id, "op")?;
    std::fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    let directory = PrivateDirectory::create(root.join("client-pending"))?;
    let path = directory.path().join(format!("{operation_id}.json"));
    save_json_private(&path, body)?;
    Ok(path)
}

pub(crate) fn save_json_private(path: &Path, value: &Value) -> Result<()> {
    private_io::write_new(path, &serde_json::to_vec_pretty(value)?)
}

#[cfg(test)]
#[path = "../../../test/admin/unit/pending/tests.rs"]
mod tests;
