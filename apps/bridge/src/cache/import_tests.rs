use super::*;
use std::io::{Cursor, Read};

#[test]
fn private_import_is_kept_only_after_successful_handoff() {
    let root = tempfile::tempdir().unwrap();
    let imports = root.path().join("imports");
    let staged = stage_import(&imports, &mut Cursor::new(b"private file")).unwrap();
    let temporary = staged.0.to_path_buf();
    assert!(temporary.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&temporary).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    drop(staged);
    assert!(!temporary.exists());
    let path = stage_import(&imports, &mut Cursor::new(b"complete file"))
        .unwrap()
        .keep()
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"complete file");
}

#[test]
fn provider_read_failure_does_not_leave_a_partial_import() {
    struct Failing(bool);
    impl Read for Failing {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            if self.0 {
                return Err(std::io::Error::other("provider failed"));
            }
            self.0 = true;
            bytes[..3].copy_from_slice(b"abc");
            Ok(3)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let imports = root.path().join("imports");
    assert!(stage_import(&imports, &mut Failing(false)).is_err());
    assert_eq!(std::fs::read_dir(imports).unwrap().count(), 0);
}

#[tokio::test]
async fn cancelling_the_async_caller_cleans_up_the_detached_provider_copy() {
    struct Held {
        started: Option<tokio::sync::oneshot::Sender<()>>,
        release: std::sync::mpsc::Receiver<()>,
        bytes: Cursor<&'static [u8]>,
    }
    impl Read for Held {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            if let Some(started) = self.started.take() {
                started.send(()).unwrap();
                self.release
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap();
            }
            self.bytes.read(bytes)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let imports = root.path().join("imports");
    let directory = imports.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let task = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            stage_import(
                &directory,
                &mut Held {
                    started: Some(started),
                    release: wait,
                    bytes: Cursor::new(b"complete private bytes"),
                },
            )
        })
        .await
        .unwrap()
    });
    ready.await.unwrap();
    assert_eq!(std::fs::read_dir(&imports).unwrap().count(), 1);
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    release.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while std::fs::read_dir(&imports).unwrap().count() != 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
