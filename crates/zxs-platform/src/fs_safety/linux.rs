//! Linux-only openat2 root policy and typed object/lock boundaries.
use crate::fs_safety::{Error, RelativePath};
use rustix::{
    fd::AsFd,
    fs::{FileType, Mode, OFlags, ResolveFlags, fstat, openat2},
};
use std::{fs::File, io, os::unix::fs::MetadataExt, path::Path};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
}
impl FileIdentity {
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
    pub fn from_fd(fd: impl AsFd) -> io::Result<Self> {
        let stat = fstat(fd)?;
        Ok(Self {
            device: stat.st_dev,
            inode: stat.st_ino,
        })
    }
}

/// Cross-mount access is an explicit filesystem capability, never a product-name branch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MountPolicy {
    SameMount,
    AllowNestedMounts,
}

#[derive(Debug)]
pub struct OpenAt2Root {
    root: File,
    identity: FileIdentity,
    resolve: ResolveFlags,
}
impl OpenAt2Root {
    pub fn open(path: &Path, mounts: MountPolicy) -> Result<Self, Error> {
        let root = crate::fs_safety::unix::open_directory(path)?;
        Self::from_directory(root, mounts)
    }

    pub fn from_directory(root: File, mounts: MountPolicy) -> Result<Self, Error> {
        let stat = fstat(&root).map_err(io::Error::from)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
            return Err(Error::UnsafeFileType("root".into()));
        }
        let mut resolve = ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS;
        if mounts == MountPolicy::SameMount {
            resolve |= ResolveFlags::NO_XDEV;
        }
        // Probe the required primitive immediately, with no openat/path fallback.
        openat2(
            &root,
            ".",
            OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            resolve,
        )
        .map_err(io::Error::from)?;
        Ok(Self {
            root,
            identity: FileIdentity {
                device: stat.st_dev,
                inode: stat.st_ino,
            },
            resolve,
        })
    }
    pub const fn identity(&self) -> FileIdentity {
        self.identity
    }
    pub fn directory(&self) -> &File {
        &self.root
    }
    pub fn into_directory(self) -> File {
        self.root
    }

    /// Open only a single-linked regular file, refusing links in every component.
    /// NONBLOCK prevents a malicious FIFO from hanging before the type check.
    pub fn open_file(&self, path: &RelativePath, write: bool) -> Result<File, Error> {
        let access = if write { OFlags::RDWR } else { OFlags::RDONLY };
        let file = File::from(
            openat2(
                &self.root,
                path.as_path(),
                access | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
                self.resolve | ResolveFlags::NO_SYMLINKS,
            )
            .map_err(io::Error::from)?,
        );
        SingleLinkRequirement::check(&file)?;
        Ok(file)
    }
    pub fn identity_of(&self, path: &RelativePath) -> Result<FileIdentity, Error> {
        Ok(FileIdentity::from_fd(self.open_file(path, false)?)?)
    }
}

pub struct SingleLinkRequirement;
impl SingleLinkRequirement {
    pub fn check(fd: impl AsFd) -> Result<FileIdentity, Error> {
        let stat = fstat(fd).map_err(io::Error::from)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err(Error::UnsafeFileType("opened object".into()));
        }
        if stat.st_nlink != 1 {
            return Err(Error::MultipleLinks("opened object".into()));
        }
        Ok(FileIdentity {
            device: stat.st_dev,
            inode: stat.st_ino,
        })
    }
}

/// Holds a duplicate of an already-anchored directory, never a separate lock pathname.
#[derive(Debug)]
pub struct AdvisoryLock {
    _directory: File,
}
impl AdvisoryLock {
    pub fn directory(directory: &File) -> Result<Self, Error> {
        if !directory.metadata()?.is_dir() {
            return Err(Error::UnsafeFileType("lock anchor".into()));
        }
        let directory = directory.try_clone()?;
        directory.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => Error::AlreadyLocked("directory".into()),
            std::fs::TryLockError::Error(error) => Error::Io(error),
        })?;
        Ok(Self {
            _directory: directory,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::{fs::symlink, net::UnixListener},
    };
    #[test]
    fn rejects_root_and_intermediate_links_and_all_non_regular_files() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("file"), b"original").unwrap();
        fs::create_dir(temp.path().join("dir")).unwrap();
        symlink(temp.path(), temp.path().join("alias")).unwrap();
        assert!(OpenAt2Root::open(&temp.path().join("alias"), MountPolicy::SameMount).is_err());
        let root = OpenAt2Root::open(temp.path(), MountPolicy::SameMount).unwrap();
        symlink("file", temp.path().join("link")).unwrap();
        fs::hard_link(temp.path().join("file"), temp.path().join("hard")).unwrap();
        rustix::fs::mknodat(
            root.directory(),
            "fifo",
            FileType::Fifo,
            Mode::from_raw_mode(0o600),
            0,
        )
        .unwrap();
        let _socket = UnixListener::bind(temp.path().join("socket")).unwrap();
        for name in [
            "dir",
            "link",
            "hard",
            "file",
            "alias/file",
            "fifo",
            "socket",
        ] {
            assert!(
                root.open_file(&RelativePath::new(name).unwrap(), false)
                    .is_err(),
                "{name}"
            );
        }
        assert_eq!(fs::read(temp.path().join("file")).unwrap(), b"original");
    }
    #[test]
    fn root_identity_and_exclusive_lock_survive_directory_rebinding() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("root");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("file"), b"old").unwrap();
        let root = OpenAt2Root::open(&path, MountPolicy::AllowNestedMounts).unwrap();
        let lock = AdvisoryLock::directory(root.directory()).unwrap();
        let second = OpenAt2Root::open(&path, MountPolicy::AllowNestedMounts).unwrap();
        assert!(AdvisoryLock::directory(second.directory()).is_err());
        let moved = temp.path().join("moved");
        fs::rename(&path, &moved).unwrap();
        fs::create_dir(&path).unwrap();
        fs::write(path.join("file"), b"new").unwrap();
        assert_eq!(
            root.identity(),
            FileIdentity::from_metadata(&fs::metadata(moved).unwrap())
        );
        let mut old = root
            .open_file(&RelativePath::new("file").unwrap(), false)
            .unwrap();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut old, &mut bytes).unwrap();
        assert_eq!(bytes, b"old");
        drop(lock);
        drop(root);
        AdvisoryLock::directory(second.directory()).unwrap();
    }
}
