//! Use the same cache key for initial download and verified reuse. Separating
//! version directories preserves the user's filename and its full name budget.
use crate::BridgeError;
use std::path::{Path, PathBuf};

/// Keep ownership across a blocking provider copy and its asynchronous handoff.
/// If the caller disappears, the returned value is dropped and the file removed.
pub struct StagedImport(tempfile::TempPath);
impl StagedImport {
    pub fn keep(self) -> Result<PathBuf, BridgeError> {
        self.0
            .keep()
            .map_err(|error| zxs_client::Error::Io(error.error).into())
    }
}

pub fn stage_import(
    directory: &Path,
    source: &mut impl std::io::Read,
) -> Result<StagedImport, BridgeError> {
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent).map_err(zxs_client::Error::Io)?;
    }
    private_directory(directory)?;
    let mut file = tempfile::Builder::new()
        .prefix("import-")
        .tempfile_in(directory)
        .map_err(zxs_client::Error::Io)?;
    std::io::copy(source, file.as_file_mut()).map_err(zxs_client::Error::Io)?;
    file.as_file().sync_all().map_err(zxs_client::Error::Io)?;
    Ok(StagedImport(file.into_temp_path()))
}

fn private_directory(path: &Path) -> Result<(), BridgeError> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(zxs_client::Error::Io(error).into()),
    }
    let metadata = std::fs::symlink_metadata(path).map_err(zxs_client::Error::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(BridgeError {
            code: "CACHE_SCOPE".into(),
            message: "Cache subtree must be a real directory".into(),
            retryable: false,
            status: 0,
        });
    }
    Ok(())
}

pub fn cache_target(directory: &Path, version: &str, name: &str) -> Result<PathBuf, BridgeError> {
    zxs_contract::validate_id(version, "ver").map_err(|error| BridgeError {
        code: "INVALID_VERSION".into(),
        message: error.message,
        retryable: false,
        status: 0,
    })?;
    zxs_naming::validate_name(name).map_err(|error| BridgeError {
        code: "INVALID_FILENAME".into(),
        message: error.message,
        retryable: false,
        status: 0,
    })?;
    Ok(directory.join("versions").join(version).join(name))
}

/// Create private version directories while refusing symlinked cache subtrees.
pub fn prepare_cache_target(
    directory: &Path,
    version: &str,
    name: &str,
) -> Result<PathBuf, BridgeError> {
    cache_target(directory, version, name)?;
    std::fs::create_dir_all(directory).map_err(zxs_client::Error::Io)?;
    let directory = directory.canonicalize().map_err(zxs_client::Error::Io)?;
    let target = cache_target(&directory, version, name)?;
    for path in [
        directory.join("versions"),
        directory.join("versions").join(version),
    ] {
        private_directory(&path)?;
    }
    // A cache collision is reusable only for an ordinary file. In particular,
    // never follow a leaf symlink or let verification block on a FIFO/device.
    match std::fs::symlink_metadata(&target) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err(BridgeError {
                code: "CACHE_SCOPE".into(),
                message: "Cache entry must be a regular file".into(),
                retryable: false,
                status: 0,
            });
        }
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(zxs_client::Error::Io(error).into()),
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    const VERSION: &str = "ver_0123456789abcdef0123456789abcdef";
    #[test]
    fn a_full_length_user_filename_does_not_grow_when_cached() {
        let name = "a".repeat(240);
        let path = cache_target(Path::new("cache"), VERSION, &name).unwrap();
        assert_eq!(path.file_name().unwrap(), name.as_str());
        assert_eq!(path.parent().unwrap().file_name().unwrap(), VERSION);
    }
    #[test]
    fn path_components_and_windows_streams_cannot_escape_the_cache_key() {
        for name in [
            "../outside",
            "a/b",
            "a\\b",
            "a:stream",
            "",
            ".",
            "..",
            "CON",
        ] {
            assert_eq!(
                cache_target(Path::new("cache"), VERSION, name)
                    .unwrap_err()
                    .code,
                "INVALID_FILENAME"
            );
        }
        assert_eq!(
            cache_target(Path::new("cache"), "../../outside", "file.txt")
                .unwrap_err()
                .code,
            "INVALID_VERSION"
        );
    }
    #[test]
    fn version_directories_are_private_and_do_not_follow_external_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        let target = prepare_cache_target(directory.path(), VERSION, "file.txt").unwrap();
        assert!(target.parent().unwrap().is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::{PermissionsExt, symlink};
            assert_eq!(
                std::fs::metadata(target.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            let linked = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            symlink(outside.path(), linked.path().join("versions")).unwrap();
            assert_eq!(
                prepare_cache_target(linked.path(), VERSION, "file.txt")
                    .unwrap_err()
                    .code,
                "CACHE_SCOPE"
            );
            assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
        }
    }
}

#[cfg(test)]
mod import_tests;

#[cfg(test)]
mod entry_tests {
    use super::*;
    const VERSION: &str = "ver_0123456789abcdef0123456789abcdef";

    #[test]
    fn only_regular_existing_entries_are_reusable() {
        let root = tempfile::tempdir().unwrap();
        let target = prepare_cache_target(root.path(), VERSION, "file.txt").unwrap();
        std::fs::create_dir(&target).unwrap();
        assert_eq!(
            prepare_cache_target(root.path(), VERSION, "file.txt")
                .unwrap_err()
                .code,
            "CACHE_SCOPE"
        );
        std::fs::remove_dir(&target).unwrap();
        std::fs::write(&target, b"verified later").unwrap();
        assert_eq!(
            prepare_cache_target(root.path(), VERSION, "file.txt").unwrap(),
            target
        );
    }

    #[cfg(unix)]
    #[test]
    fn leaf_symlinks_are_rejected_even_inside_cache_or_when_dangling() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let target = prepare_cache_target(root.path(), VERSION, "file.txt").unwrap();
        let other = root.path().join("other");
        for exists in [false, true] {
            if exists {
                std::fs::write(&other, b"other").unwrap();
            }
            symlink(&other, &target).unwrap();
            assert_eq!(
                prepare_cache_target(root.path(), VERSION, "file.txt")
                    .unwrap_err()
                    .code,
                "CACHE_SCOPE"
            );
            std::fs::remove_file(&target).unwrap();
        }
    }
}
