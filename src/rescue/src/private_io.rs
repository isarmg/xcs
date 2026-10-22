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
}

/// Read an explicitly selected owned draft without following links. Drafts
/// need not have secret-file permissions, but still use the regular-file and
/// single-link checks of the descriptor-anchored I/O layer.
impl AnchoredPath {
    fn read_owned_bytes(&self, maximum: usize) -> Result<Vec<u8>> {
        let file = self.directory.open_file(&self.name.as_relative(), false)?;
        let metadata = file.metadata()?;
        if metadata.uid() != rustix::process::geteuid().as_raw() {
            return Err("configuration draft must belong to this user".into());
        }
        if metadata.len() > maximum as u64 {
            return Err("configuration draft exceeds its byte limit".into());
        }
        let mut bytes = Vec::new();
        file.take((maximum as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() > maximum {
            return Err("configuration draft exceeds its byte limit".into());
        }
        Ok(bytes)
    }
}
pub(crate) fn read_owned_bytes(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    AnchoredPath::open(path)?.read_owned_bytes(maximum)
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
    let name = format!(".zxs-rescue-{}.tmp", authentication::random_token()?);
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
#[path = "../../../test/rescue/unit/private_io/tests.rs"]
mod tests;
