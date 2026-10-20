//! Descriptor-anchored CLI files. Output parents may be public (for example
//! /tmp); the file itself is owner-only, single-linked and never overwritten.
use crate::Result;
use platform_io::fs_safety::{
    EntryName,
    linux::{MountPolicy, OpenAt2Root},
};
use rustix::fs::{AtFlags, Mode, OFlags, RenameFlags, openat, renameat, renameat_with, unlinkat};
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};

struct AnchoredPath {
    directory: OpenAt2Root,
    name: EntryName,
}
impl AnchoredPath {
    fn open(path: &Path) -> Result<Self> {
        let name = EntryName::new(path.file_name().ok_or("file name is required")?)?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        // Resolve the caller-selected parent once. All subsequent file opens,
        // publication and cleanup use the held directory descriptor.
        let parent = parent.canonicalize()?;
        Ok(Self {
            directory: OpenAt2Root::open(&parent, MountPolicy::AllowNestedMounts)?,
            name,
        })
    }
    fn read_private(&self, maximum: usize) -> Result<String> {
        let file = self.directory.open_file(&self.name.as_relative(), false)?;
        let metadata = file.metadata()?;
        if metadata.uid() != rustix::process::geteuid().as_raw()
            || !matches!(metadata.mode() & 0o7777, 0o400 | 0o600)
        {
            return Err("secret file must belong to this user and have mode 0400 or 0600".into());
        }
        let mut bytes = Vec::new();
        file.take((maximum as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() > maximum {
            return Err("secret input exceeds its byte limit".into());
        }
        Ok(String::from_utf8(bytes)?)
    }
}

pub(crate) fn read_private(path: &Path, maximum: usize) -> Result<String> {
    AnchoredPath::open(path)?.read_private(maximum)
}

struct StagingCleanup<'a> {
    directory: &'a File,
    name: String,
    armed: bool,
}
impl Drop for StagingCleanup<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = unlinkat(self.directory, self.name.as_str(), AtFlags::empty());
        }
    }
}

/// Publish only a complete 0600 file; existing files, directories and links win.
/// A directory-sync failure after rename preserves PublishedDurabilityUnknown.
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic(path, bytes, false)
}

/// Update an explicit owned regular draft file without following any link.
pub(crate) fn replace_private(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic(path, bytes, true)
}

fn write_atomic(path: &Path, bytes: &[u8], replace: bool) -> Result<()> {
    let target = AnchoredPath::open(path)?;
    let directory = target.directory.directory();
    if replace {
        let existing = target
            .directory
            .open_file(&target.name.as_relative(), false)?;
        if existing.metadata()?.uid() != rustix::process::geteuid().as_raw() {
            return Err("configuration draft must belong to this user".into());
        }
    }
    let name = format!(".storage-admin-{}.tmp", authentication::random_token()?);
    let mut file = File::from(openat(
        directory,
        name.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )?);
    // Arm cleanup only after the exclusive inode was created successfully.
    let mut cleanup = StagingCleanup {
        directory,
        name,
        armed: true,
    };
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    if replace {
        renameat(
            directory,
            cleanup.name.as_str(),
            directory,
            target.name.as_os_str(),
        )?;
    } else {
        renameat_with(
            directory,
            cleanup.name.as_str(),
            directory,
            target.name.as_os_str(),
            RenameFlags::NOREPLACE,
        )?;
    }
    // The temporary name no longer belongs to us after publication. Disarm
    // before syncing so a later user of that name cannot be removed by Drop.
    cleanup.armed = false;
    directory
        .sync_all()
        .map_err(platform_io::fs_safety::Error::PublishedDurabilityUnknown)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        sync::Arc,
    };

    #[test]
    fn private_reads_validate_the_opened_file_and_bound_its_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("credential");
        write_new(&path, b"fixture-private-value").unwrap();
        assert_eq!(read_private(&path, 21).unwrap(), "fixture-private-value");
        assert!(read_private(&path, 20).is_err());
        for mode in [0o644, 0o640, 0o700, 0o2600] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            assert!(read_private(&path, 1024).is_err(), "{mode:o}");
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(read_private(&path, 1024).is_ok());
        symlink(&path, directory.path().join("alias")).unwrap();
        assert!(read_private(&directory.path().join("alias"), 1024).is_err());
        fs::hard_link(&path, directory.path().join("hard")).unwrap();
        assert!(read_private(&path, 1024).is_err());
        assert!(read_private(directory.path(), 1024).is_err());
    }

    #[test]
    fn held_parent_is_not_reopened_after_directory_rebinding() {
        let temporary = tempfile::tempdir().unwrap();
        let parent = temporary.path().join("parent");
        fs::create_dir(&parent).unwrap();
        write_new(&parent.join("credential"), b"original").unwrap();
        let held = AnchoredPath::open(&parent.join("credential")).unwrap();
        let moved = temporary.path().join("moved");
        fs::rename(&parent, &moved).unwrap();
        fs::create_dir(&parent).unwrap();
        write_new(&parent.join("credential"), b"replacement").unwrap();
        assert_eq!(held.read_private(1024).unwrap(), "original");
    }

    #[test]
    fn atomic_create_is_private_no_clobber_and_cleans_failed_staging() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("published");
        write_new(&path, b"complete").unwrap();
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(metadata.mode() & 0o7777, 0o600);
        assert_eq!(metadata.nlink(), 1);
        assert!(write_new(&path, b"overwrite").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"complete");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        let alias = directory.path().join("alias");
        symlink(&path, &alias).unwrap();
        assert!(write_new(&alias, b"follow link").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"complete");
        let occupied = directory.path().join("occupied");
        fs::create_dir(&occupied).unwrap();
        assert!(write_new(&occupied, b"must not publish").is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
    }

    #[test]
    fn concurrent_private_publication_has_exactly_one_complete_winner() {
        let directory = tempfile::tempdir().unwrap();
        let path = Arc::new(directory.path().join("published"));
        let threads = [b"first".as_slice(), b"second".as_slice()]
            .into_iter()
            .map(|bytes| {
                let path = path.clone();
                std::thread::spawn(move || write_new(&path, bytes).is_ok())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            threads
                .into_iter()
                .map(|thread| thread.join().unwrap())
                .filter(|won| *won)
                .count(),
            1
        );
        let bytes = fs::read(&*path).unwrap();
        assert!(bytes == b"first" || bytes == b"second");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn published_staging_cleanup_preserves_a_reused_temporary_name() {
        let temporary = tempfile::tempdir().unwrap();
        let directory =
            OpenAt2Root::open(temporary.path(), MountPolicy::AllowNestedMounts).unwrap();
        let name = "temporary";
        fs::write(temporary.path().join(name), b"published").unwrap();
        let mut cleanup = StagingCleanup {
            directory: directory.directory(),
            name: name.to_owned(),
            armed: true,
        };
        renameat(directory.directory(), name, directory.directory(), "final").unwrap();
        cleanup.armed = false;
        fs::write(temporary.path().join(name), b"another creator").unwrap();
        drop(cleanup);
        assert_eq!(
            fs::read(temporary.path().join(name)).unwrap(),
            b"another creator"
        );
        assert_eq!(
            fs::read(temporary.path().join("final")).unwrap(),
            b"published"
        );
    }
    #[test]
    fn replacement_updates_only_owned_regular_files_without_following_links() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("draft");
        write_new(&original, b"before").unwrap();
        replace_private(&original, b"after").unwrap();
        assert_eq!(fs::read(&original).unwrap(), b"after");
        let alias = directory.path().join("alias");
        symlink(&original, &alias).unwrap();
        assert!(replace_private(&alias, b"must not follow").is_err());
        assert_eq!(fs::read(&original).unwrap(), b"after");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
    #[test]
    fn administrator_cannot_accidentally_read_another_owners_secret() {
        if rustix::process::geteuid().as_raw() != 0 {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("foreign");
        write_new(&path, b"fixture-value").unwrap();
        let file = File::open(&path).unwrap();
        rustix::fs::fchown(&file, Some(rustix::process::Uid::from_raw(65534)), None).unwrap();
        assert!(read_private(&path, 1024).is_err());
    }
}
