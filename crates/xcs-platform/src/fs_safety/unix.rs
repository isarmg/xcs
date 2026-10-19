//! Portable Unix descriptor-relative primitives; intentionally does not need openat2.
use super::{
    AdvisoryLock, EntryName, Error, FileEntry, InventoryLimits, PrivateDirectory, RelativePath,
    temporary_name,
};
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{
        AtFlags, FileType, Mode, OFlags, fstat, mkdirat, open, openat, renameat, statat, unlinkat,
    },
    process::geteuid,
};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    path::{Component, Path},
};

const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

pub(super) fn open_directory(path: &Path) -> Result<File, Error> {
    if !path.is_absolute() {
        return Err(Error::AbsolutePathRequired(path.to_path_buf()));
    }
    let mut fd = open("/", DIRECTORY_FLAGS, Mode::empty()).map_err(io::Error::from)?;
    for component in path.components() {
        match component {
            Component::RootDir => (),
            Component::Normal(name) => {
                fd = openat(&fd, name, DIRECTORY_FLAGS, Mode::empty()).map_err(io::Error::from)?;
            }
            _ => return Err(Error::UnsafeRelativePath(path.to_path_buf())),
        }
    }
    Ok(File::from(fd))
}

pub(super) fn open_private_directory(path: &Path) -> Result<PrivateDirectory, Error> {
    let directory = open_directory(path)?;
    validate_private_directory(&directory, path)?;
    Ok(PrivateDirectory {
        path: path.to_path_buf(),
        directory,
        administration_owner: None,
    })
}

pub(super) fn open_administrative_directory(path: &Path) -> Result<PrivateDirectory, Error> {
    let directory = open_directory(path)?;
    let stat = fstat(&directory).map_err(io::Error::from)?;
    let caller = geteuid().as_raw();
    if (caller != 0 && caller != stat.st_uid) || stat.st_mode & 0o7777 != 0o700 {
        return Err(Error::UnsafePermissions(path.to_path_buf()));
    }
    Ok(PrivateDirectory {
        path: path.to_path_buf(),
        directory,
        administration_owner: Some((stat.st_uid, stat.st_gid)),
    })
}

fn validate_private_directory(directory: &File, path: &Path) -> Result<(), Error> {
    let stat = fstat(directory).map_err(io::Error::from)?;
    if stat.st_uid != geteuid().as_raw() || stat.st_mode & 0o7777 != 0o700 {
        return Err(Error::UnsafePermissions(path.to_path_buf()));
    }
    Ok(())
}

pub(super) fn create_private_directory(path: &Path) -> Result<PrivateDirectory, Error> {
    if !path.is_absolute() {
        return Err(Error::AbsolutePathRequired(path.to_path_buf()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| Error::UnsafeRelativePath(path.to_path_buf()))?;
    let name = path
        .file_name()
        .ok_or_else(|| Error::UnsafeRelativePath(path.to_path_buf()))?;
    let parent = open_directory(parent)?;
    create_private_at(&parent, name, path, None)
}

pub(super) fn create_private_child(
    parent: &PrivateDirectory,
    name: &EntryName,
) -> Result<PrivateDirectory, Error> {
    create_private_at(
        &parent.directory,
        name.as_os_str(),
        &parent.path.join(name.as_path()),
        parent.administration_owner,
    )
}

fn create_private_at(
    parent: &File,
    name: &OsStr,
    path: &Path,
    administration_owner: Option<(u32, u32)>,
) -> Result<PrivateDirectory, Error> {
    let created = match mkdirat(parent, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => true,
        Err(rustix::io::Errno::EXIST) => false,
        Err(error) => return Err(io::Error::from(error).into()),
    };
    let directory =
        File::from(openat(parent, name, DIRECTORY_FLAGS, Mode::empty()).map_err(io::Error::from)?);
    if let Some((uid, gid)) = administration_owner {
        if created {
            // A service-owned parent may be renamed concurrently. Only the
            // newly created inode still owned by this caller may be re-owned.
            validate_private_directory(&directory, path)?;
            rustix::fs::fchown(
                &directory,
                Some(rustix::process::Uid::from_raw(uid)),
                Some(rustix::process::Gid::from_raw(gid)),
            )
            .map_err(io::Error::from)?;
        }
        validate_administration(&directory, path, uid, gid)?;
    } else {
        validate_private_directory(&directory, path)?;
    }
    // Never chmod an existing path: even a rejected symlink must have no effect.
    if created {
        directory.sync_all()?;
        parent.sync_all()?;
    }
    Ok(PrivateDirectory {
        path: path.to_path_buf(),
        directory,
        administration_owner,
    })
}

fn entry_name(path: &RelativePath) -> Result<&OsStr, Error> {
    if path.as_path().components().count() != 1 {
        return Err(Error::UnsafeRelativePath(path.as_path().to_path_buf()));
    }
    path.as_path()
        .file_name()
        .ok_or_else(|| Error::UnsafeRelativePath(path.as_path().to_path_buf()))
}

pub(super) fn sync_file_and_parent(path: &Path) -> Result<(), Error> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::UnsafeRelativePath(path.to_path_buf()))?;
    let name = path
        .file_name()
        .ok_or_else(|| Error::UnsafeRelativePath(path.to_path_buf()))?;
    let directory = open_directory(parent)?;
    let file = File::from(
        openat(
            &directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)?,
    );
    regular_single_link(&file, path)?;
    file.sync_all()?;
    directory.sync_all()?;
    Ok(())
}

pub(super) fn files(
    directory: &PrivateDirectory,
    limits: InventoryLimits,
) -> Result<Vec<FileEntry>, Error> {
    let mut entries = rustix::fs::Dir::read_from(&directory.directory).map_err(io::Error::from)?;
    let mut result = Vec::new();
    let mut total_bytes = 0u64;
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(io::Error::from)?;
        let name = entry.file_name();
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        if result.len() >= limits.max_entries {
            return Err(Error::BudgetExceeded);
        }
        use std::os::unix::ffi::OsStrExt;
        let name = EntryName::new(OsStr::from_bytes(name.to_bytes()))?;
        let file = open_regular(directory, &name)?;
        let bytes = file.metadata()?.len();
        total_bytes = total_bytes
            .checked_add(bytes)
            .ok_or(Error::BudgetExceeded)?;
        if total_bytes > limits.max_total_bytes {
            return Err(Error::BudgetExceeded);
        }
        result.push(FileEntry { name, bytes });
    }
    Ok(result)
}

fn open_regular(directory: &PrivateDirectory, name: &EntryName) -> Result<File, Error> {
    verify_administration(directory)?;
    let file = File::from(
        openat(
            &directory.directory,
            name.as_os_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)?,
    );
    let identity = regular_single_link(&file, name.as_path())?;
    validate_administrative_file(directory, &identity, name.as_path())?;
    let current = statat(
        &directory.directory,
        name.as_os_str(),
        AtFlags::SYMLINK_NOFOLLOW,
    )
    .map_err(io::Error::from)?;
    if (identity.st_dev, identity.st_ino) != (current.st_dev, current.st_ino) {
        return Err(Error::IdentityChanged(name.as_path().to_path_buf()));
    }
    Ok(file)
}

pub(super) fn read_bounded(
    directory: &PrivateDirectory,
    name: &EntryName,
    max_bytes: usize,
) -> Result<Vec<u8>, Error> {
    let file = open_regular(directory, name)?;
    if file.metadata()?.len() > max_bytes as u64 {
        return Err(Error::BudgetExceeded);
    }
    let mut bytes = Vec::new();
    file.take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(Error::BudgetExceeded);
    }
    Ok(bytes)
}

pub(super) fn remove_file(directory: &PrivateDirectory, name: &EntryName) -> Result<(), Error> {
    let _file = open_regular(directory, name)?;
    unlinkat(&directory.directory, name.as_os_str(), AtFlags::empty()).map_err(io::Error::from)?;
    directory.sync()
}

fn regular_single_link(fd: impl AsFd, path: &Path) -> Result<rustix::fs::Stat, Error> {
    let stat = fstat(fd).map_err(io::Error::from)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(Error::UnsafeFileType(path.to_path_buf()));
    }
    if stat.st_nlink != 1 {
        return Err(Error::MultipleLinks(path.to_path_buf()));
    }
    Ok(stat)
}

fn validate_destination(directory: &PrivateDirectory, name: &OsStr) -> Result<(), Error> {
    verify_administration(directory)?;
    match statat(&directory.directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => {
            if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
                return Err(Error::UnsafeFileType(name.into()));
            }
            if stat.st_nlink != 1 {
                return Err(Error::MultipleLinks(name.into()));
            }
            validate_administrative_file(directory, &stat, Path::new(name))?;
            Ok(())
        }
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(error) => Err(io::Error::from(error).into()),
    }
}

pub(super) fn atomic_replace(
    directory: &PrivateDirectory,
    destination: &RelativePath,
    bytes: &[u8],
) -> Result<(), Error> {
    atomic_write(directory, destination, bytes, false)
}

pub(super) fn atomic_create(
    directory: &PrivateDirectory,
    destination: &EntryName,
    bytes: &[u8],
) -> Result<(), Error> {
    atomic_write(directory, &destination.as_relative(), bytes, true)
}

fn atomic_write(
    directory: &PrivateDirectory,
    destination: &RelativePath,
    bytes: &[u8],
    create_only: bool,
) -> Result<(), Error> {
    let name = entry_name(destination)?;
    validate_destination(directory, name)?;
    let temporary = temporary_name()?;
    let fd: OwnedFd = openat(
        &directory.directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(io::Error::from)?;
    let mut file = File::from(fd);
    let result = (|| {
        if let Some((uid, gid)) = directory.administration_owner {
            // Only the exclusive new temporary descriptor is re-owned. The
            // destination inode is never chowned, chmodded, or followed.
            rustix::fs::fchown(
                &file,
                Some(rustix::process::Uid::from_raw(uid)),
                Some(rustix::process::Gid::from_raw(gid)),
            )
            .map_err(io::Error::from)?;
        }
        let identity = regular_single_link(&file, Path::new(&temporary))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        validate_destination(directory, name)?;
        if create_only {
            no_clobber_publish(directory, &RelativePath::new(&temporary)?, destination)?;
        } else {
            renameat(
                &directory.directory,
                temporary.as_str(),
                &directory.directory,
                name,
            )
            .map_err(io::Error::from)?;
        }
        let published = statat(&directory.directory, name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(io::Error::from)?;
        if (identity.st_dev, identity.st_ino) != (published.st_dev, published.st_ino) {
            return Err(Error::IdentityChanged(destination.as_path().to_path_buf()));
        }
        directory
            .directory
            .sync_all()
            .map_err(Error::PublishedDurabilityUnknown)
    })();
    if result.is_err() {
        // Our exclusive temporary lives under the held private directory handle.
        let _ = unlinkat(&directory.directory, temporary.as_str(), AtFlags::empty());
    }
    result
}

pub(super) fn advisory_lock(
    directory: &PrivateDirectory,
    name: &RelativePath,
) -> Result<AdvisoryLock, Error> {
    verify_administration(directory)?;
    let name = entry_name(name)?;
    let path = Path::new(name);
    let flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let opened = openat(
        &directory.directory,
        name,
        flags | OFlags::CREATE | OFlags::EXCL,
        Mode::from_raw_mode(0o600),
    );
    let (file, created) = match opened {
        Ok(fd) => (File::from(fd), true),
        Err(rustix::io::Errno::EXIST) => (
            File::from(
                openat(&directory.directory, name, flags, Mode::empty())
                    .map_err(io::Error::from)?,
            ),
            false,
        ),
        Err(error) => return Err(io::Error::from(error).into()),
    };
    if created && let Some((uid, gid)) = directory.administration_owner {
        rustix::fs::fchown(
            &file,
            Some(rustix::process::Uid::from_raw(uid)),
            Some(rustix::process::Gid::from_raw(gid)),
        )
        .map_err(io::Error::from)?;
    }
    let identity = regular_single_link(&file, path)?;
    let owner = directory
        .administration_owner
        .map_or(geteuid().as_raw(), |(uid, _)| uid);
    if identity.st_uid != owner || identity.st_mode & 0o7777 != 0o600 {
        return Err(Error::UnsafePermissions(path.to_path_buf()));
    }
    file.try_lock().map_err(|error| match error {
        std::fs::TryLockError::WouldBlock => Error::AlreadyLocked(path.to_path_buf()),
        std::fs::TryLockError::Error(error) => Error::Io(error),
    })?;
    let current =
        statat(&directory.directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
    if (identity.st_dev, identity.st_ino) != (current.st_dev, current.st_ino) {
        return Err(Error::IdentityChanged(path.to_path_buf()));
    }
    file.sync_all()?;
    directory.sync()?;
    Ok(AdvisoryLock { _file: file })
}

fn validate_administration(file: &File, path: &Path, uid: u32, gid: u32) -> Result<(), Error> {
    let stat = fstat(file).map_err(io::Error::from)?;
    if stat.st_uid != uid || stat.st_gid != gid || stat.st_mode & 0o7777 != 0o700 {
        return Err(Error::UnsafePermissions(path.to_path_buf()));
    }
    let caller = geteuid().as_raw();
    if caller != 0 && caller != uid {
        return Err(Error::UnsafePermissions(path.to_path_buf()));
    }
    Ok(())
}

fn verify_administration(directory: &PrivateDirectory) -> Result<(), Error> {
    if let Some((uid, gid)) = directory.administration_owner {
        validate_administration(&directory.directory, directory.path(), uid, gid)?;
    }
    Ok(())
}

fn validate_administrative_file(
    directory: &PrivateDirectory,
    stat: &rustix::fs::Stat,
    path: &Path,
) -> Result<(), Error> {
    if let Some((uid, gid)) = directory.administration_owner
        && (stat.st_uid != uid || stat.st_gid != gid || stat.st_mode & 0o7777 != 0o600)
    {
        return Err(Error::UnsafePermissions(path.to_path_buf()));
    }
    Ok(())
}

pub(super) fn no_clobber_publish(
    directory: &PrivateDirectory,
    source: &RelativePath,
    destination: &RelativePath,
) -> Result<(), Error> {
    let source = entry_name(source)?;
    let destination = entry_name(destination)?;
    let file = File::from(
        openat(
            &directory.directory,
            source,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)?,
    );
    let identity = regular_single_link(&file, Path::new(source))?;
    file.sync_all()?;
    #[cfg(target_os = "linux")]
    rustix::fs::renameat_with(
        &directory.directory,
        source,
        &directory.directory,
        destination,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            Error::DestinationExists(destination.into())
        } else {
            io::Error::from(error).into()
        }
    })?;
    #[cfg(not(target_os = "linux"))]
    {
        rustix::fs::linkat(
            &directory.directory,
            source,
            &directory.directory,
            destination,
            AtFlags::empty(),
        )
        .map_err(io::Error::from)?;
        directory
            .directory
            .sync_all()
            .map_err(Error::PublishedDurabilityUnknown)?;
        unlinkat(&directory.directory, source, AtFlags::empty())
            .map_err(|error| Error::PublishedDurabilityUnknown(error.into()))?;
    }
    let current = statat(&directory.directory, destination, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|error| Error::PublishedDurabilityUnknown(error.into()))?;
    if (identity.st_dev, identity.st_ino) != (current.st_dev, current.st_ino) {
        return Err(Error::IdentityChanged(destination.into()));
    }
    directory
        .directory
        .sync_all()
        .map_err(Error::PublishedDurabilityUnknown)
}

/// DFS keeps at most 128 directory handles, independent of directory width.
pub(super) fn bounded_inventory(
    path: &Path,
    limits: crate::fs_safety::InventoryLimits,
) -> Result<crate::fs_safety::DirectoryInventory, Error> {
    use rustix::fs::Dir;
    let root = open_directory(path)?;
    let mut stack = vec![Dir::new(root).map_err(io::Error::from)?];
    let mut result = crate::fs_safety::DirectoryInventory {
        entries: 0,
        total_bytes: 0,
    };
    while let Some(current) = stack.last_mut() {
        let Some(entry) = current.read() else {
            stack.pop();
            continue;
        };
        let entry = entry.map_err(io::Error::from)?;
        let name = entry.file_name();
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        result.entries = result.entries.checked_add(1).ok_or(Error::BudgetExceeded)?;
        if result.entries > limits.max_entries {
            return Err(Error::BudgetExceeded);
        }
        let parent = current.fd().map_err(io::Error::from)?;
        let stat = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        let kind = FileType::from_raw_mode(stat.st_mode);
        if !matches!(kind, FileType::Directory | FileType::RegularFile) {
            return Err(Error::UnsafeFileType(
                name.to_string_lossy().into_owned().into(),
            ));
        }
        let fd = openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)?;
        let opened = fstat(&fd).map_err(io::Error::from)?;
        if (opened.st_dev, opened.st_ino, opened.st_mode)
            != (stat.st_dev, stat.st_ino, stat.st_mode)
        {
            return Err(Error::IdentityChanged(
                name.to_string_lossy().into_owned().into(),
            ));
        }
        if kind == FileType::RegularFile && opened.st_nlink != 1 {
            return Err(Error::MultipleLinks(
                name.to_string_lossy().into_owned().into(),
            ));
        }
        result.total_bytes = result
            .total_bytes
            .checked_add(u64::try_from(opened.st_size).map_err(|_| Error::BudgetExceeded)?)
            .ok_or(Error::BudgetExceeded)?;
        if result.total_bytes > limits.max_total_bytes {
            return Err(Error::BudgetExceeded);
        }
        if kind == FileType::Directory {
            if stack.len() >= 128 {
                return Err(Error::BudgetExceeded);
            }
            stack.push(Dir::new(fd).map_err(io::Error::from)?);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_safety::AtomicFile;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };

    #[test]
    fn administrative_atomic_files_keep_service_owner_and_refuse_unsafe_targets() {
        use std::os::unix::fs::{MetadataExt, chown};
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("state");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = if geteuid().as_raw() == 0 {
            65534
        } else {
            geteuid().as_raw()
        };
        let gid = if geteuid().as_raw() == 0 {
            65534
        } else {
            rustix::process::getegid().as_raw()
        };
        if geteuid().as_raw() == 0 {
            chown(&path, Some(uid), Some(gid)).unwrap();
            assert!(PrivateDirectory::open_existing(&path).is_err());
        }
        let directory = PrivateDirectory::open_for_administration(&path).unwrap();
        let pending = RelativePath::new("pending.json").unwrap();
        AtomicFile::replace(&directory, &pending, b"first").unwrap();
        AtomicFile::replace(&directory, &pending, b"second").unwrap();
        let metadata = fs::metadata(path.join("pending.json")).unwrap();
        assert_eq!(
            (
                metadata.uid(),
                metadata.gid(),
                metadata.mode() & 0o7777,
                metadata.nlink()
            ),
            (uid, gid, 0o600, 1)
        );
        assert_eq!(
            directory
                .read_bounded(&EntryName::new("pending.json").unwrap(), 6)
                .unwrap(),
            b"second"
        );
        let link = path.join("link");
        symlink(path.join("pending.json"), &link).unwrap();
        assert!(
            AtomicFile::replace(&directory, &RelativePath::new("link").unwrap(), b"bad").is_err()
        );
        fs::set_permissions(path.join("pending.json"), fs::Permissions::from_mode(0o640)).unwrap();
        assert!(AtomicFile::replace(&directory, &pending, b"bad").is_err());
        assert_eq!(fs::read(path.join("pending.json")).unwrap(), b"second");
        fs::set_permissions(path.join("pending.json"), fs::Permissions::from_mode(0o600)).unwrap();
        directory
            .remove_file(&EntryName::new("pending.json").unwrap())
            .unwrap();
        assert!(!path.join("pending.json").exists());
    }

    #[test]
    fn rejecting_private_directory_does_not_chmod_a_symlink_target_or_existing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let victim = temp.path().join("victim");
        fs::create_dir(&victim).unwrap();
        fs::set_permissions(&victim, fs::Permissions::from_mode(0o755)).unwrap();
        let link = temp.path().join("link");
        symlink(&victim, &link).unwrap();
        assert!(PrivateDirectory::create(&link).is_err());
        assert!(PrivateDirectory::create(&victim).is_err());
        assert_eq!(
            fs::metadata(&victim).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        assert!(PrivateDirectory::create(link.join("nested")).is_err());
        assert!(!victim.join("nested").exists());
    }

    #[test]
    fn held_private_directory_survives_path_rebinding_without_touching_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("state");
        let root = PrivateDirectory::create(&original).unwrap();
        let moved = temp.path().join("moved");
        fs::rename(&original, &moved).unwrap();
        fs::create_dir(&original).unwrap();
        AtomicFile::replace(&root, &RelativePath::new("entry").unwrap(), b"private").unwrap();
        assert_eq!(fs::read(moved.join("entry")).unwrap(), b"private");
        assert!(!original.join("entry").exists());
        assert_eq!(
            fs::metadata(moved.join("entry"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        AdvisoryLock::acquire(&root, &RelativePath::new("lock").unwrap()).unwrap();
        assert!(!original.join("lock").exists());
    }

    #[test]
    fn writes_and_locks_refuse_links_and_nested_aliases() {
        let temp = tempfile::tempdir().unwrap();
        let root = PrivateDirectory::create(temp.path().join("state")).unwrap();
        let victim = temp.path().join("victim");
        fs::write(&victim, b"untouched").unwrap();
        symlink(&victim, root.path().join("link")).unwrap();
        fs::hard_link(&victim, root.path().join("hard")).unwrap();
        symlink(temp.path(), root.path().join("escape")).unwrap();
        for name in ["link", "hard", "escape/victim"] {
            let name = RelativePath::new(name).unwrap();
            assert!(AtomicFile::replace(&root, &name, b"changed").is_err());
            assert!(AdvisoryLock::acquire(&root, &name).is_err());
            assert!(crate::fs_safety::sync_file_and_parent(&root.resolve(&name)).is_err());
        }
        assert_eq!(fs::read(victim).unwrap(), b"untouched");
    }

    #[test]
    fn typed_entry_io_is_bounded_and_create_never_replaces_existing_content() {
        let temp = tempfile::tempdir().unwrap();
        let directory = PrivateDirectory::create(temp.path().join("state")).unwrap();
        for invalid in ["", ".", "..", "a/b", "a/", "a\0b"] {
            assert!(EntryName::new(invalid).is_err(), "{invalid:?}");
        }
        let entry = EntryName::new("current").unwrap();
        AtomicFile::create(&directory, &entry, b"original").unwrap();
        assert!(AtomicFile::create(&directory, &entry, b"replace").is_err());
        assert_eq!(directory.read_bounded(&entry, 8).unwrap(), b"original");
        assert!(matches!(
            directory.read_bounded(&entry, 7),
            Err(Error::BudgetExceeded)
        ));
        let entries = directory
            .files(InventoryLimits {
                max_entries: 1,
                max_total_bytes: 8,
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].bytes, 8);
        assert!(matches!(
            directory.files(InventoryLimits {
                max_entries: 0,
                max_total_bytes: 8
            }),
            Err(Error::BudgetExceeded)
        ));
        assert!(matches!(
            directory.files(InventoryLimits {
                max_entries: 1,
                max_total_bytes: 7
            }),
            Err(Error::BudgetExceeded)
        ));
        let path = directory.path().join(entry.as_path());
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_len(8 * 1024 * 1024 * 1024)
            .unwrap();
        assert!(matches!(
            directory.read_bounded(&entry, 1024),
            Err(Error::BudgetExceeded)
        ));
        directory.remove_file(&entry).unwrap();
        assert!(
            directory
                .files(InventoryLimits {
                    max_entries: 0,
                    max_total_bytes: 0
                })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn no_clobber_has_one_winner_and_never_replaces_an_occupant() {
        let temp = tempfile::tempdir().unwrap();
        let root =
            std::sync::Arc::new(PrivateDirectory::create(temp.path().join("state")).unwrap());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|value| {
                let root = root.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let source = RelativePath::new(value).unwrap();
                    AtomicFile::replace(&root, &source, value.as_bytes()).unwrap();
                    barrier.wait();
                    crate::fs_safety::NoClobberPublish::publish(
                        &root,
                        &source,
                        &RelativePath::new("current").unwrap(),
                    )
                    .is_ok()
                })
            })
            .collect();
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| usize::from(worker.join().unwrap()))
                .sum::<usize>(),
            1
        );
        let bytes = fs::read(root.path().join("current")).unwrap();
        assert!(bytes == b"first" || bytes == b"second");
        let victim = temp.path().join("victim");
        fs::write(&victim, b"untouched").unwrap();
        symlink(&victim, root.path().join("occupied")).unwrap();
        let current = RelativePath::new("current").unwrap();
        assert!(
            crate::fs_safety::NoClobberPublish::publish(
                &root,
                &current,
                &RelativePath::new("occupied").unwrap()
            )
            .is_err()
        );
        assert_eq!(fs::read(victim).unwrap(), b"untouched");
        assert_eq!(fs::read(root.resolve(&current)).unwrap(), bytes);
    }

    #[test]
    fn inventory_rejects_linked_root_special_files_and_excessive_depth() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let alias = temp.path().join("alias");
        symlink(&root, &alias).unwrap();
        let limits = crate::fs_safety::InventoryLimits {
            max_entries: 1000,
            max_total_bytes: 16 * 1024 * 1024,
        };
        assert!(bounded_inventory(&alias, limits).is_err());
        let mut parent = root.clone();
        for _ in 0..128 {
            parent = parent.join("d");
            fs::create_dir(&parent).unwrap();
        }
        assert!(matches!(
            bounded_inventory(&root, limits),
            Err(Error::BudgetExceeded)
        ));
        let specials = tempfile::tempdir().unwrap();
        let _listener =
            std::os::unix::net::UnixListener::bind(specials.path().join("socket")).unwrap();
        assert!(matches!(
            bounded_inventory(specials.path(), limits),
            Err(Error::UnsafeFileType(_))
        ));
    }
}
