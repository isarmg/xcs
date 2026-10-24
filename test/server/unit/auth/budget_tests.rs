//! Credential write limits must match restart admission without partial changes.
use super::*;

const STORE_LIMIT: usize = 4 * 1024 * 1024;

#[test]
fn authoritative_snapshot_recovers_an_oversized_safe_local_file() {
    let root = crate::tests::private_tempdir();
    let mut store =
        AuthStore::initialize(root.path(), "admin", "credential-budget-password").unwrap();
    let login = store.login("admin", "credential-budget-password").unwrap();
    let snapshot = store.snapshot().unwrap();
    drop(store);
    let local = root.path().join("auth.json");
    // Preserve the admitted regular file's private mode and single-link identity.
    fs::write(&local, vec![b' '; STORE_LIMIT + 1]).unwrap();
    assert_eq!(AuthStore::open(root.path()).err().unwrap().status, 400);
    assert_eq!(fs::metadata(&local).unwrap().len(), STORE_LIMIT as u64 + 1);
    let recovered = AuthStore::open_with_snapshot(root.path(), Some(snapshot.clone())).unwrap();
    assert_eq!(recovered.snapshot().unwrap(), snapshot);
    assert!(recovered.authenticate(&login.token).is_ok());
    assert!(fs::metadata(&local).unwrap().len() < STORE_LIMIT as u64);
}

#[test]
fn oversized_authoritative_snapshot_never_replaces_existing_credentials() {
    let root = crate::tests::private_tempdir();
    let mut store =
        AuthStore::initialize(root.path(), "admin", "credential-budget-password").unwrap();
    let login = store.login("admin", "credential-budget-password").unwrap();
    let mut snapshot = store.snapshot().unwrap();
    drop(store);
    let local = root.path().join("auth.json");
    let previous = fs::read(&local).unwrap();
    snapshot["administrator"]["username"] = json_codec::json!("r".repeat(STORE_LIMIT));
    assert_eq!(
        AuthStore::open_with_snapshot(root.path(), Some(snapshot))
            .err()
            .unwrap()
            .status,
        400
    );
    assert_eq!(fs::read(local).unwrap(), previous);
    assert!(
        AuthStore::open(root.path())
            .unwrap()
            .authenticate(&login.token)
            .is_ok()
    );
}

#[cfg(unix)]
#[test]
fn authoritative_recovery_preserves_local_file_safety_checks() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    for kind in ["symlink", "hard_link", "public_mode"] {
        let root = crate::tests::private_tempdir();
        let store =
            AuthStore::initialize(root.path(), "admin", "credential-budget-password").unwrap();
        let snapshot = store.snapshot().unwrap();
        drop(store);
        let local = root.path().join("auth.json");
        let previous = fs::read(&local).unwrap();
        let alias = root.path().join("untouched.json");
        match kind {
            "symlink" => {
                fs::rename(&local, &alias).unwrap();
                symlink(&alias, &local).unwrap();
            }
            "hard_link" => fs::hard_link(&local, &alias).unwrap(),
            "public_mode" => {
                fs::set_permissions(&local, fs::Permissions::from_mode(0o644)).unwrap()
            }
            _ => unreachable!(),
        }
        assert_eq!(
            AuthStore::open_with_snapshot(root.path(), Some(snapshot))
                .err()
                .unwrap()
                .status,
            400,
            "{kind}"
        );
        assert_eq!(fs::read(&local).unwrap(), previous, "{kind}");
    }
}

#[test]
fn an_over_capacity_snapshot_is_rejected_before_any_local_publication() {
    let root = crate::tests::private_tempdir();
    let mut store =
        AuthStore::initialize(root.path(), "owner", "credential-budget-password").unwrap();
    let login = store.login("owner", "credential-budget-password").unwrap();
    let mut snapshot = store.snapshot().unwrap();
    drop(store);
    let path = root.path().join("auth.json");
    let before = fs::read(&path).unwrap();
    for index in 0..MAX_SESSIONS {
        snapshot["sessions"][format!("{index:064x}")] =
            json_codec::json!({"generation":"1","expires_at":"1"});
    }
    assert!(AuthStore::open_with_snapshot(root.path(), Some(snapshot)).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(
        AuthStore::open(root.path())
            .unwrap()
            .authenticate(&login.token)
            .is_ok()
    );
}

#[test]
fn maximum_valid_singleton_store_is_bounded_and_reopens() {
    let root = crate::tests::private_tempdir();
    let username = "x".repeat(primitives::ADMINISTRATOR_USERNAME_MAX_BYTES);
    let mut store =
        AuthStore::initialize(root.path(), &username, "credential-budget-password").unwrap();
    for index in 0..MAX_SESSIONS {
        store.database_mut().unwrap().sessions.insert(
            format!("{index:064x}"),
            Session {
                generation: u64::MAX,
                expires_at: u64::MAX,
            },
        );
    }
    store.persist(false).unwrap();
    let snapshot = store.snapshot().unwrap();
    assert!(json_codec::to_vec(&snapshot).unwrap().len() < STORE_LIMIT);
    drop(store);
    assert_eq!(
        AuthStore::open(root.path()).unwrap().snapshot().unwrap(),
        snapshot
    );
}
