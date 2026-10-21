#[cfg(test)]
mod tests {
    use std::env;
    use std::error;
    use std::fs::{self, File};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::result;

    use super::is_same_file;

    type Result<T> = result::Result<T, Box<error::Error + Send + Sync>>;

    /// Create an error from a format!-like syntax.
    macro_rules! err {
        ($($tt:tt)*) => {
            Box::<error::Error + Send + Sync>::from(format!($($tt)*))
        }
    }

    /// A simple wrapper for creating a temporary directory that is
    /// automatically deleted when it's dropped.
    ///
    /// We use this in lieu of tempfile because tempfile brings in too many
    /// dependencies.
    #[derive(Debug)]
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    impl TempDir {
        /// Create a new empty temporary directory under the system's
        /// configured temporary directory.
        fn new() -> Result<TempDir> {
            #![allow(deprecated)]

            use std::sync::atomic::{
                AtomicUsize, Ordering, ATOMIC_USIZE_INIT,
            };

            static TRIES: usize = 100;
            static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;

            let tmpdir = env::temp_dir();
            for _ in 0..TRIES {
                let count = COUNTER.fetch_add(1, Ordering::SeqCst);
                let path = tmpdir.join("rust-walkdir").join(count.to_string());
                if path.is_dir() {
                    continue;
                }
                fs::create_dir_all(&path).map_err(|e| {
                    err!("failed to create {}: {}", path.display(), e)
                })?;
                return Ok(TempDir(path));
            }
            Err(err!("failed to create temp dir after {} tries", TRIES))
        }

        /// Return the underlying path to this temporary directory.
        fn path(&self) -> &Path {
            &self.0
        }
    }

    fn tmpdir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[cfg(unix)]
    pub fn soft_link_dir<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        use std::os::unix::fs::symlink;
        symlink(src, dst)
    }

    #[cfg(unix)]
    pub fn soft_link_file<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        soft_link_dir(src, dst)
    }

    #[cfg(windows)]
    pub fn soft_link_dir<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        use std::os::windows::fs::symlink_dir;
        symlink_dir(src, dst)
    }

    #[cfg(windows)]
    pub fn soft_link_file<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        use std::os::windows::fs::symlink_file;
        symlink_file(src, dst)
    }

    // These tests are rather uninteresting. The really interesting tests
    // would stress the edge cases. On Unix, this might be comparing two files
    // on different mount points with the same inode number. On Windows, this
    // might be comparing two files whose file indices are the same on file
    // systems where such things aren't guaranteed to be unique.
    //
    // Alas, I don't know how to create those environmental conditions. ---AG

    #[test]
    fn same_file_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("a")).unwrap());
    }

    #[test]
    fn same_dir_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        fs::create_dir(dir.join("a")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("a")).unwrap());
    }

    #[test]
    fn not_same_file_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        File::create(dir.join("b")).unwrap();
        assert!(!is_same_file(dir.join("a"), dir.join("b")).unwrap());
    }

    #[test]
    fn not_same_dir_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        fs::create_dir(dir.join("a")).unwrap();
        fs::create_dir(dir.join("b")).unwrap();
        assert!(!is_same_file(dir.join("a"), dir.join("b")).unwrap());
    }

    #[test]
    fn same_file_hard() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        fs::hard_link(dir.join("a"), dir.join("alink")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("alink")).unwrap());
    }

    #[test]
    fn same_file_soft() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        soft_link_file(dir.join("a"), dir.join("alink")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("alink")).unwrap());
    }

    #[test]
    fn same_dir_soft() {
        let tdir = tmpdir();
        let dir = tdir.path();

        fs::create_dir(dir.join("a")).unwrap();
        soft_link_dir(dir.join("a"), dir.join("alink")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("alink")).unwrap());
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<super::Handle>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<super::Handle>();
    }
}
