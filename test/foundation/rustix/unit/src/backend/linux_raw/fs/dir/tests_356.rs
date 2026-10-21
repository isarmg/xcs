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
        // will cause the subsequent `getdents64` to fail.
        crate::io::dup2(&file_fd, &mut dir.fd).unwrap();

        assert!(matches!(dir.next(), Some(Err(_))));
        assert!(dir.next().is_none());
    }
}
