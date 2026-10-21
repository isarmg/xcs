#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, format, fs, process};

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_env_var_is_not_set() {
        init();
        let mut inner = KeyLogFileInner::new(None);
        assert!(
            inner
                .try_write("label", b"random", b"secret")
                .is_ok()
        );
    }

    #[test]
    fn test_env_var_cannot_be_opened() {
        init();
        let mut inner = KeyLogFileInner::new(Some("/dev/does-not-exist".into()));
        assert!(
            inner
                .try_write("label", b"random", b"secret")
                .is_ok()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_env_var_cannot_be_written() {
        init();
        let mut inner = KeyLogFileInner::new(Some("/dev/full".into()));
        assert!(
            inner
                .try_write("label", b"random", b"secret")
                .is_err()
        );
    }

    #[test]
    fn test_created_file_has_owner_only_permissions() {
        let path = env::temp_dir().join(format!(
            "rustls-keylog-perm-{}-{}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_file(&path);

        let inner = KeyLogFileInner::new(Some(path.clone().into()));
        assert!(inner.file.is_some(), "key log file should open");

        let mode = fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        let _ = fs::remove_file(&path);

        assert_eq!(
            mode, 0o600,
            "SSLKEYLOGFILE must be created with mode 0o600, got {mode:#o}"
        );
    }
}
