//! Complete no-clobber publication of configuration and retry request files.
use crate::{Result, private_io};
use serde_json::Value;
use std::path::{Path, PathBuf};
use zxs_platform::fs_safety::PrivateDirectory;

pub(crate) fn save_pending(root: &Path, operation_id: &str, body: &Value) -> Result<PathBuf> {
    zxs_core::validate_id(operation_id, "op")?;
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
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };

    #[test]
    fn pending_requests_are_private_complete_and_never_replace_an_existing_request() {
        let root = tempfile::tempdir().unwrap();
        let operation = zxs_core::new_id("op");
        let body = json!({"operation_id":operation,"fencing_token":u64::MAX.to_string()});
        let path = save_pending(root.path(), &operation, &body).unwrap();
        assert_eq!(
            fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
            0o600
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
            body
        );
        assert!(save_pending(root.path(), &operation, &json!({"changed":true})).is_err());
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
            body
        );
    }

    #[test]
    fn pending_creation_does_not_repair_or_follow_an_existing_unsafe_directory() {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let pending = root.path().join("client-pending");
        symlink(external.path(), &pending).unwrap();
        assert!(save_pending(root.path(), &zxs_core::new_id("op"), &json!({})).is_err());
        assert_eq!(fs::read_dir(external.path()).unwrap().count(), 0);
        fs::remove_file(&pending).unwrap();
        fs::create_dir(&pending).unwrap();
        fs::set_permissions(&pending, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(save_pending(root.path(), &zxs_core::new_id("op"), &json!({})).is_err());
        assert_eq!(
            fs::metadata(pending).unwrap().permissions().mode() & 0o7777,
            0o755
        );
    }
}
