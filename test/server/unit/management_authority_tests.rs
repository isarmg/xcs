//! Restoring storage evidence must never grant system credential authority.
use super::*;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PASSWORD: &str = "recovered-test-password";

struct Fixture {
    _root: tempfile::TempDir,
    source: PathBuf,
    data: PathBuf,
    management: PathBuf,
    disks: BTreeMap<String, PathBuf>,
    old_session: String,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source-system");
        let data = root.path().join("recovered-system");
        let management = root.path().join("management");
        let disk = root.path().join("disk");
        let mut source_auth =
            AuthStore::initialize(&root.path().join("source-auth"), "admin", PASSWORD).unwrap();
        let old_session = source_auth.login("admin", PASSWORD).unwrap().token;
        let mut engine = storage_engine::Engine::initialize(
            &source,
            storage_contract::Config::development(vec![disk.clone()]),
        )
        .unwrap();
        engine
            .persist_auth_state(
                source_auth.snapshot().unwrap(),
                "system_auth",
                &storage_engine::new_id("op"),
            )
            .unwrap();
        drop(engine);
        create_private_directory(&management).unwrap();
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(management.join("bootstrap.key"))
            .unwrap()
            .write_all(KEY.as_bytes())
            .unwrap();
        Self {
            _root: root,
            source,
            data,
            management,
            disks: BTreeMap::from([("one".into(), disk)]),
            old_session,
        }
    }

    /// Install the startup auth handle without spawning background workers, so
    /// recover_once can be checked synchronously before and after a restart.
    fn open(&self) -> (Arc<ManagementBackend>, Arc<Mutex<AuthStore>>) {
        let (backend, auth) =
            open_service_with_disks(&self.data, &self.management, self.disks.clone(), true)
                .unwrap();
        let auth = Arc::new(Mutex::new(auth));
        *backend.inner.auth.lock().unwrap() = Some(auth.clone());
        (backend, auth)
    }

    fn assert_closed(&self, backend: &ManagementBackend, auth: &Mutex<AuthStore>) {
        assert!(!auth.lock().unwrap().is_available());
        assert_eq!(
            auth.lock()
                .unwrap()
                .login("admin", PASSWORD)
                .err()
                .unwrap()
                .code,
            "RECOVERY_REQUIRED"
        );
        assert!(backend.inner.backend().is_err());
    }
}

#[test]
fn rejected_bootstrap_restore_cannot_publish_credentials_on_retry_or_restart() {
    for change in ["delete", "rotate", "expire", "revoke"] {
        let f = Fixture::new();
        let (backend, auth) = f.open();
        let key = f.management.join("bootstrap.key");
        crate::setup_recovery::after_next_restore(move || match change {
            "delete" => fs::remove_file(key).unwrap(),
            "rotate" => fs::write(key, "f".repeat(64)).unwrap(),
            "expire" => fs::File::open(key)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(
                    std::time::SystemTime::now() - std::time::Duration::from_secs(1801),
                ))
                .unwrap(),
            "revoke" => fs::set_permissions(key, fs::Permissions::from_mode(0o644)).unwrap(),
            _ => unreachable!(),
        });
        assert!(
            backend
                .setup_operation(
                    "restore_system",
                    json!({"disk_ids":["one"],"accept_stale_credentials":true}),
                    Some(KEY),
                    None,
                )
                .is_err()
        );
        assert!(f.data.join("config.json").exists());
        assert!(!f.data.join("management-authority.json").exists());
        assert!(!f.management.join("management-auth.json").exists());
        for _ in 0..2 {
            assert_eq!(
                backend.inner.recover_once().unwrap_err().code,
                "AUTH_RECOVERY_REQUIRED"
            );
            f.assert_closed(&backend, &auth);
            assert!(!f.management.join("management-auth.json").exists());
            assert!(!f.data.join("management-authority.json").exists());
        }
        drop(backend);
        drop(auth);
        let (backend, auth) = f.open();
        assert_eq!(
            backend.inner.recover_once().unwrap_err().code,
            "AUTH_RECOVERY_REQUIRED"
        );
        f.assert_closed(&backend, &auth);
        assert!(!f.management.join("management-auth.json").exists());
        assert!(!f.data.join("management-authority.json").exists());
    }
}

#[test]
fn missing_or_corrupt_local_credentials_never_migrate_even_without_a_marker() {
    for corrupt in [false, true] {
        let mut f = Fixture::new();
        f.data = f.source.clone();
        if corrupt {
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(f.management.join("management-auth.json"))
                .unwrap()
                .write_all(b"invalid credentials")
                .unwrap();
        }
        for _ in 0..2 {
            let (backend, auth) = f.open();
            assert_eq!(
                backend.inner.recover_once().unwrap_err().code,
                "AUTH_RECOVERY_REQUIRED"
            );
            f.assert_closed(&backend, &auth);
            assert!(!f.data.join("management-authority.json").exists());
            if corrupt {
                assert_eq!(
                    fs::read(f.management.join("management-auth.json")).unwrap(),
                    b"invalid credentials"
                );
            } else {
                assert!(!f.management.join("management-auth.json").exists());
            }
        }
    }
}

#[test]
fn existing_valid_local_authority_wins_over_a_recovered_snapshot() {
    let mut f = Fixture::new();
    f.data = f.source.clone();
    drop(
        AuthStore::initialize_management(&f.management, "admin", "current-test-password").unwrap(),
    );
    let (backend, auth) = f.open();
    backend.inner.recover_once().unwrap();
    assert!(auth.lock().unwrap().login("admin", PASSWORD).is_err());
    assert!(
        auth.lock()
            .unwrap()
            .login("admin", "current-test-password")
            .is_ok()
    );
    assert!(auth.lock().unwrap().authenticate(&f.old_session).is_err());
    assert!(f.data.join("management-authority.json").exists());
    assert!(backend.inner.backend().is_ok());
}

#[test]
fn explicitly_authorized_restore_installs_credentials_but_not_old_sessions() {
    for authenticated in [false, true] {
        let f = Fixture::new();
        let password = if authenticated {
            drop(
                AuthStore::initialize_management(&f.management, "admin", "current-test-password")
                    .unwrap(),
            );
            "current-test-password"
        } else {
            PASSWORD
        };
        let (backend, auth) = f.open();
        let identity = authenticated.then(|| {
            let mut auth = auth.lock().unwrap();
            let login = auth.login("admin", password).unwrap();
            auth.authenticate(&login.token).unwrap()
        });
        backend
            .setup_operation(
                "restore_system",
                json!({"disk_ids":["one"],"accept_stale_credentials":!authenticated}),
                (!authenticated).then_some(KEY),
                identity.as_ref(),
            )
            .unwrap();
        assert!(auth.lock().unwrap().login("admin", password).is_ok());
        assert!(auth.lock().unwrap().authenticate(&f.old_session).is_err());
        if authenticated {
            assert!(auth.lock().unwrap().login("admin", PASSWORD).is_err());
        }
        assert!(f.data.join("management-authority.json").exists());
        assert!(f.management.join("management-auth.json").exists());
        // Wait only for the successful background recovery to release the
        // engine; revocation injection and rejection checks use no timing race.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while backend.inner.recovering.load(Ordering::SeqCst)
            || Arc::strong_count(&backend.inner) > 1
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(backend);
        drop(auth);
        let (backend, auth) = f.open();
        backend.inner.recover_once().unwrap();
        assert!(auth.lock().unwrap().login("admin", password).is_ok());
        assert!(auth.lock().unwrap().authenticate(&f.old_session).is_err());
    }
}
