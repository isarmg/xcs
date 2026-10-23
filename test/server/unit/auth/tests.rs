use super::*;
#[test]
fn self_password_rotation_preserves_scope_revokes_sessions_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
    let repositories = BTreeSet::from(["repo-private".to_owned()]);
    store
        .upsert_account(
            "reader",
            Some("reader-old-password"),
            false,
            repositories.clone(),
            false,
        )
        .unwrap();
    let first = store.login("reader", "reader-old-password").unwrap();
    let second = store.login("reader", "reader-old-password").unwrap();
    store
        .change_password(&first.token, "reader-old-password", "reader-new-password")
        .unwrap();
    for token in [&first.token, &second.token] {
        assert_eq!(store.authenticate(token).unwrap_err().code, "UNAUTHORIZED");
    }
    assert!(store.login("reader", "reader-old-password").is_err());
    drop(store);
    let mut store = AuthStore::open(dir.path()).unwrap();
    let session = store.login("reader", "reader-new-password").unwrap();
    let identity = store.authenticate(&session.token).unwrap();
    assert_eq!(identity.principal, "reader");
    assert!(!identity.admin);
    assert_eq!(identity.repositories, repositories);
    assert!(store.login("admin", "a-long-test-password").is_ok());
}

#[test]
fn rejected_password_rotation_keeps_credentials_and_session_intact() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
    let session = store.login("admin", "a-long-test-password").unwrap();
    let before = store.snapshot().unwrap();
    assert_eq!(
        store
            .change_password(&session.token, "incorrect-password", "a-new-test-password")
            .unwrap_err()
            .code,
        "CURRENT_PASSWORD_INVALID"
    );
    assert!(
        store
            .change_password(&session.token, "a-long-test-password", "short")
            .is_err()
    );
    assert_eq!(store.snapshot().unwrap(), before);
    assert!(store.authenticate(&session.token).is_ok());
    drop(store);
    let mut store = AuthStore::open(dir.path()).unwrap();
    assert!(store.login("admin", "a-long-test-password").is_ok());
}

#[test]
fn persisted_sessions_are_revoked_by_account_generation() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
    let login = store.login("admin", "a-long-test-password").unwrap();
    drop(store);
    let mut store = AuthStore::open(dir.path()).unwrap();
    assert!(store.authenticate(&login.token).is_ok());
    store
        .upsert_account("admin", None, true, BTreeSet::new(), false)
        .unwrap();
    assert_eq!(
        store.authenticate(&login.token).unwrap_err().code,
        "UNAUTHORIZED"
    );
}
#[test]
fn cannot_disable_last_administrator() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
    assert!(
        store
            .upsert_account("admin", None, false, BTreeSet::new(), false)
            .is_err()
    );
}
#[test]
fn missing_local_credentials_restore_only_from_authoritative_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
    let session = store.login("admin", "a-long-test-password").unwrap();
    let snapshot = store.snapshot().unwrap();
    drop(store);
    fs::remove_file(root.path().join("auth.json")).unwrap();
    assert!(AuthStore::open(root.path()).is_err());
    let restored = AuthStore::open_with_snapshot(root.path(), Some(snapshot)).unwrap();
    assert!(restored.authenticate(&session.token).is_ok());
}
#[test]
fn uncertain_backup_blocks_authentication_until_recovery() {
    let root = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    store
        .attach_backup(Arc::new(move |_| {
            if count.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(())
            } else {
                Err(ApiError::new(
                    503,
                    "COMMIT_IN_DOUBT",
                    "publication interrupted",
                ))
            }
        }))
        .unwrap();
    assert!(store.login("admin", "a-long-test-password").is_err());
    assert_eq!(
        store.identity_for_principal("admin").unwrap_err().code,
        "RECOVERY_REQUIRED"
    );
    let before = store.snapshot().unwrap();
    assert_eq!(
        store
            .upsert_account(
                "queued",
                Some("queued-test-password"),
                false,
                BTreeSet::new(),
                false
            )
            .unwrap_err()
            .code,
        "RECOVERY_REQUIRED"
    );
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn detached_login_verification_cannot_cross_account_changes_or_recovery() {
    let root = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
    store
        .upsert_account(
            "reader",
            Some("reader-old-password"),
            false,
            BTreeSet::new(),
            false,
        )
        .unwrap();
    // Verify first, then mutate as a second request could while hashing runs.
    let proof = store
        .login_challenge("reader")
        .unwrap()
        .verify("reader-old-password")
        .unwrap();
    store
        .upsert_account(
            "reader",
            Some("reader-new-password"),
            false,
            BTreeSet::new(),
            false,
        )
        .unwrap();
    assert_eq!(store.finish_login(proof).err().unwrap().status, 401);
    let proof = store
        .login_challenge("reader")
        .unwrap()
        .verify("reader-new-password")
        .unwrap();
    store
        .upsert_account("reader", None, true, BTreeSet::new(), false)
        .unwrap();
    assert_eq!(store.finish_login(proof).err().unwrap().status, 401);
    let proof = store
        .login_challenge("reader")
        .unwrap()
        .verify("reader-new-password")
        .unwrap();
    store
        .upsert_account("reader", None, true, BTreeSet::new(), true)
        .unwrap();
    assert_eq!(store.finish_login(proof).err().unwrap().status, 401);
    let proof = store
        .login_challenge("admin")
        .unwrap()
        .verify("a-long-test-password")
        .unwrap();
    store.blocked.store(true, Ordering::SeqCst);
    assert_eq!(
        store.finish_login(proof).err().unwrap().code,
        "RECOVERY_REQUIRED"
    );
    assert!(store.db.sessions.is_empty());
}

#[test]
fn detached_login_challenge_keeps_unknown_disabled_and_wrong_password_rejected() {
    let root = tempfile::tempdir().unwrap();
    let mut store = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
    assert!(
        store
            .login_challenge("unknown")
            .unwrap()
            .verify("a-long-test-password")
            .is_err()
    );
    assert!(
        store
            .login_challenge("admin")
            .unwrap()
            .verify("wrong-password")
            .is_err()
    );
    store
        .upsert_account(
            "disabled",
            Some("a-long-test-password"),
            false,
            BTreeSet::new(),
            true,
        )
        .unwrap();
    assert!(
        store
            .login_challenge("disabled")
            .unwrap()
            .verify("a-long-test-password")
            .is_err()
    );
    let proof = store
        .login_challenge("admin")
        .unwrap()
        .verify("a-long-test-password")
        .unwrap();
    let session = store.finish_login(proof).unwrap();
    assert!(store.authenticate(&session.token).unwrap().admin);
}

#[test]
fn account_refresh_preserves_internal_smb_binding_without_serializing_it() {
    let root = tempfile::tempdir().unwrap();
    let store = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
    let mut caller = store.identity_for_principal("admin").unwrap();
    caller.smb_binding = Some(SmbBinding {
        repository_id: "repository-one".into(),
        branch_id: "branch-one".into(),
        generation: 7,
    });
    let current = store.revalidate_identity(&caller).unwrap();
    assert_eq!(current.smb_binding, caller.smb_binding);
    assert!(
        json_codec::to_value(&current)
            .unwrap()
            .get("smb_binding")
            .is_none()
    );
}
