#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_iterator_handles_io_errors() {
        // create a dir, keep the FD, then delete the dir
        let tmp = tempfile::tempdir().unwrap();
        let fd = crate::fs::openat(
            crate::fs::CWD,
            tmp.path(),
            crate::fs::OFlags::RDONLY | crate::fs::OFlags::CLOEXEC,
            crate::fs::Mode::empty(),
        )
        .unwrap();

        let file_fd = crate::fs::openat(
            &fd,
            tmp.path().join("test.txt"),
            crate::fs::OFlags::WRONLY | crate::fs::OFlags::CREATE,
            crate::fs::Mode::RWXU,
        )
        .unwrap();

        let mut dir = Dir::read_from(&fd).unwrap();

        // Reach inside the `Dir` and replace its directory with a file, which
        // will cause the subsequent `readdir` to fail.
        unsafe {
            let raw_fd = c::dirfd(dir.libc_dir.as_ptr());
            let mut owned_fd: crate::fd::OwnedFd = crate::fd::FromRawFd::from_raw_fd(raw_fd);
            crate::io::dup2(&file_fd, &mut owned_fd).unwrap();
            core::mem::forget(owned_fd);
        }

        // FreeBSD and macOS seem to read some directory entries before we call
        // `.next()`.
        #[cfg(any(apple, freebsdlike))]
        {
            dir.rewind();
        }

        assert!(matches!(dir.next(), Some(Err(_))));
        assert!(dir.next().is_none());
    }
}
