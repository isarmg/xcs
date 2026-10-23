//! Seed session records directly: capacity regressions must not hash thousands
//! of passwords or depend on timing races.
use super::*;

fn proof(store: &AuthStore, username: &str) -> VerifiedLogin {
    let account = &store.db.accounts[username];
    VerifiedLogin {
        username: username.into(),
        password_hash: account.password_hash.clone(),
        generation: account.generation,
    }
}

fn add_account(store: &mut AuthStore, username: &str, disabled: bool) {
    let mut account = store.db.accounts["admin"].clone();
    account.username = username.into();
    account.admin = false;
    account.disabled = disabled;
    store.db.accounts.insert(username.into(), account);
}

fn seed(store: &mut AuthStore, index: usize, username: &str, generation: u64, expires_at: u64) {
    store.db.sessions.insert(
        format!("{index:064x}"),
        Session {
            username: username.into(),
            generation,
            expires_at,
        },
    );
}

fn seed_invalid(store: &mut AuthStore, count: usize) {
    add_account(store, "disabled", true);
    for index in 0..count {
        let (username, generation, expires_at) = match index % 4 {
            0 => ("admin", 1, 0),
            1 => ("admin", 0, u64::MAX),
            2 => ("disabled", 1, u64::MAX),
            _ => ("missing", 1, u64::MAX),
        };
        seed(store, index, username, generation, expires_at);
    }
}

#[test]
fn invalid_sessions_do_not_exhaust_global_capacity_after_restart() {
    let root = crate::tests::private_tempdir();
    let mut store = AuthStore::initialize(root.path(), "admin", "capacity-test-password").unwrap();
    seed_invalid(&mut store, MAX_SESSIONS);
    store.persist(false).unwrap();
    drop(store);
    let mut store = AuthStore::open(root.path()).unwrap();
    let login = store.finish_login(proof(&store, "admin")).unwrap();
    assert_eq!(store.db.sessions.len(), 1);
    drop(store);
    let store = AuthStore::open(root.path()).unwrap();
    assert_eq!(store.db.sessions.len(), 1);
    assert!(store.authenticate(&login.token).is_ok());
}

#[test]
fn one_account_cannot_consume_other_accounts_session_capacity() {
    let root = crate::tests::private_tempdir();
    let mut store = AuthStore::initialize(root.path(), "admin", "capacity-test-password").unwrap();
    add_account(&mut store, "reader", false);
    for index in 0..MAX_SESSIONS_PER_ACCOUNT {
        seed(&mut store, index, "reader", 1, u64::MAX);
    }
    store.persist(false).unwrap();
    let before = store.snapshot().unwrap();
    assert_eq!(
        store
            .finish_login(proof(&store, "reader"))
            .err()
            .unwrap()
            .status,
        429
    );
    assert_eq!(store.snapshot().unwrap(), before);
    assert!(store.finish_login(proof(&store, "admin")).is_ok());
    store
        .upsert_account("reader", None, false, BTreeSet::new(), false)
        .unwrap();
    assert_eq!(store.db.sessions.len(), 1);
    assert!(store.finish_login(proof(&store, "reader")).is_ok());
    drop(store);
    let store = AuthStore::open(root.path()).unwrap();
    assert_eq!(store.db.sessions.len(), 2);
}

#[test]
fn failed_login_and_account_publication_restore_pruned_sessions_exactly() {
    for update_account in [false, true] {
        let root = crate::tests::private_tempdir();
        let mut store =
            AuthStore::initialize(root.path(), "admin", "capacity-test-password").unwrap();
        let active = store.finish_login(proof(&store, "admin")).unwrap();
        seed_invalid(&mut store, 4);
        store.persist(false).unwrap();
        let before = store.snapshot().unwrap();
        let disk = fs::read(root.path().join("auth.json")).unwrap();
        store.backup = Some(Arc::new(|_| {
            Err(ApiError::conflict(
                "COMMIT_PREPARATION_BUSY",
                "injected persistence failure",
            ))
        }));
        let error = if update_account {
            store
                .upsert_account("admin", None, true, BTreeSet::new(), false)
                .unwrap_err()
        } else {
            store.finish_login(proof(&store, "admin")).err().unwrap()
        };
        assert_eq!(error.code, "COMMIT_PREPARATION_BUSY");
        assert_eq!(store.snapshot().unwrap(), before);
        assert_eq!(fs::read(root.path().join("auth.json")).unwrap(), disk);
        assert!(store.authenticate(&active.token).is_ok());
        drop(store);
        let store = AuthStore::open(root.path()).unwrap();
        assert_eq!(store.snapshot().unwrap(), before);
        assert!(store.authenticate(&active.token).is_ok());
    }
}

#[test]
fn account_updates_prune_all_invalid_records_but_keep_unrelated_sessions() {
    let root = crate::tests::private_tempdir();
    let mut store = AuthStore::initialize(root.path(), "admin", "capacity-test-password").unwrap();
    add_account(&mut store, "reader", false);
    seed_invalid(&mut store, 4);
    let admin = store.finish_login(proof(&store, "admin")).unwrap();
    let reader = store.finish_login(proof(&store, "reader")).unwrap();
    seed_invalid(&mut store, 4);
    store
        .upsert_account("reader", None, false, BTreeSet::new(), true)
        .unwrap();
    assert_eq!(store.db.sessions.len(), 1);
    assert!(store.authenticate(&admin.token).is_ok());
    assert!(store.authenticate(&reader.token).is_err());
    drop(store);
    let store = AuthStore::open(root.path()).unwrap();
    assert_eq!(store.db.sessions.len(), 1);
    assert!(store.authenticate(&admin.token).is_ok());
}

#[test]
fn valid_sessions_still_obey_global_capacity() {
    let root = crate::tests::private_tempdir();
    let mut store = AuthStore::initialize(root.path(), "admin", "capacity-test-password").unwrap();
    for index in 0..MAX_SESSIONS {
        let username = format!("reader-{}", index / MAX_SESSIONS_PER_ACCOUNT);
        if index % MAX_SESSIONS_PER_ACCOUNT == 0 {
            add_account(&mut store, &username, false);
        }
        seed(&mut store, index, &username, 1, u64::MAX);
    }
    store.persist(false).unwrap();
    let before = store.snapshot().unwrap();
    assert_eq!(
        store
            .finish_login(proof(&store, "admin"))
            .err()
            .unwrap()
            .status,
        429
    );
    assert_eq!(store.snapshot().unwrap(), before);
    drop(store);
    let store = AuthStore::open(root.path()).unwrap();
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn creating_an_account_does_not_revive_orphaned_sessions() {
    let root = crate::tests::private_tempdir();
    let mut store = AuthStore::initialize(root.path(), "admin", "capacity-test-password").unwrap();
    seed(&mut store, 0, "new-reader", 1, u64::MAX);
    store
        .upsert_account(
            "new-reader",
            Some("capacity-test-password"),
            false,
            BTreeSet::new(),
            false,
        )
        .unwrap();
    assert!(store.db.sessions.is_empty());
    drop(store);
    assert!(AuthStore::open(root.path()).unwrap().db.sessions.is_empty());
}
