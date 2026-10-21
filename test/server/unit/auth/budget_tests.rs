//! Credential write limits must match restart admission without partial changes.
use super::*;
use std::sync::atomic::AtomicUsize;

const STORE_LIMIT: usize = 4 * 1024 * 1024;

fn backup_counter(store: &mut AuthStore) -> Arc<AtomicUsize> {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    store
        .attach_backup(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }))
        .unwrap();
    calls
}

#[test]
fn authoritative_snapshot_recovers_an_oversized_safe_local_file() {
    let root = tempfile::tempdir().unwrap();
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
    let root = tempfile::tempdir().unwrap();
    let mut store =
        AuthStore::initialize(root.path(), "admin", "credential-budget-password").unwrap();
    let login = store.login("admin", "credential-budget-password").unwrap();
    let mut snapshot = store.snapshot().unwrap();
    drop(store);
    let local = root.path().join("auth.json");
    let previous = fs::read(&local).unwrap();
    snapshot["accounts"]["admin"]["repositories"] = json_codec::json!(["r".repeat(STORE_LIMIT)]);
    assert_eq!(
        AuthStore::open_with_snapshot(root.path(), Some(snapshot))
            .err()
            .unwrap()
            .status,
        429
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
        let root = tempfile::tempdir().unwrap();
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
fn oversized_account_update_is_rejected_before_backup_or_disk_changes() {
    let root = tempfile::tempdir().unwrap();
    let mut store =
        AuthStore::initialize(root.path(), "admin", "credential-budget-password").unwrap();
    let login = store.login("admin", "credential-budget-password").unwrap();
    let calls = backup_counter(&mut store);
    let snapshot = store.snapshot().unwrap();
    let disk = fs::read(root.path().join("auth.json")).unwrap();
    let failure = store
        .upsert_account(
            "admin",
            None,
            true,
            BTreeSet::from(["x".repeat(STORE_LIMIT)]),
            false,
        )
        .expect_err("a successful mutation must remain readable after restart");
    assert_eq!(failure.status, 429);
    assert_eq!(store.snapshot().unwrap(), snapshot);
    assert_eq!(fs::read(root.path().join("auth.json")).unwrap(), disk);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(store.authenticate(&login.token).is_ok());
    drop(store);
    assert!(
        AuthStore::open(root.path())
            .unwrap()
            .authenticate(&login.token)
            .is_ok()
    );
}

/// Put valid account records at the exact serialized boundary, without hundreds
/// of unrelated Argon2 evaluations or persistent engine transactions.
fn fill_to_limit(store: &mut AuthStore) {
    let prototype = store.db.accounts["admin"].clone();
    for index in 0..580 {
        let username = format!("account{index:04}");
        let mut account = prototype.clone();
        account.username = username.clone();
        account.admin = false;
        account.repositories = BTreeSet::from(["r".repeat(6900)]);
        store.db.accounts.insert(username, account);
    }
    let current = json_codec::to_vec(&store.db).unwrap().len();
    assert!(current < STORE_LIMIT);
    store.db.accounts.get_mut("admin").unwrap().repositories = BTreeSet::from([String::new()]);
    let current = json_codec::to_vec(&store.db).unwrap().len();
    store.db.accounts.get_mut("admin").unwrap().repositories =
        BTreeSet::from(["p".repeat(STORE_LIMIT - current)]);
    assert_eq!(json_codec::to_vec(&store.db).unwrap().len(), STORE_LIMIT);
}

#[test]
fn full_store_rejects_login_without_pruning_sessions_or_blocking_recovery() {
    let root = tempfile::tempdir().unwrap();
    let mut store =
        AuthStore::initialize(root.path(), "admin", "credential-budget-password").unwrap();
    let active = store.login("admin", "credential-budget-password").unwrap();
    store.db.sessions.insert(
        "0".repeat(64),
        Session {
            username: "admin".into(),
            generation: 1,
            expires_at: 0,
        },
    );
    fill_to_limit(&mut store);
    store.persist(false).unwrap();
    let calls = backup_counter(&mut store);
    let snapshot = store.snapshot().unwrap();
    let disk = fs::read(root.path().join("auth.json")).unwrap();
    // Each ordinary update fits the HTTP account-body limit; their aggregate
    // still must not exceed the credential store's restart bound.
    assert_eq!(
        store
            .upsert_account(
                "account0000",
                None,
                false,
                BTreeSet::from(["r".repeat(6901)]),
                false,
            )
            .unwrap_err()
            .status,
        429
    );
    assert_eq!(
        store
            .upsert_account(
                "new-account",
                Some("credential-budget-password"),
                false,
                BTreeSet::new(),
                false,
            )
            .unwrap_err()
            .status,
        429
    );
    // Replacing an expired session with a current expiry still grows the
    // serialized state. Failed admission must not consume that expired entry.
    let failure = store
        .login("admin", "credential-budget-password")
        .err()
        .expect("login must not publish a file larger than the reader accepts");
    assert_eq!(failure.status, 429);
    assert_eq!(store.snapshot().unwrap(), snapshot);
    assert_eq!(fs::read(root.path().join("auth.json")).unwrap(), disk);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(store.authenticate(&active.token).is_ok());
    drop(store);
    let mut restored = AuthStore::open_with_snapshot(root.path(), Some(snapshot)).unwrap();
    assert!(restored.authenticate(&active.token).is_ok());
    assert_eq!(
        fs::metadata(root.path().join("auth.json")).unwrap().len(),
        STORE_LIMIT as u64
    );
    restored.logout(&active.token).unwrap();
    assert!(
        restored
            .login("admin", "credential-budget-password")
            .is_ok()
    );
}
