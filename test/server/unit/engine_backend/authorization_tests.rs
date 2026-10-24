use super::*;
use std::collections::BTreeSet;

#[test]
fn preparation_conflicts_retain_retryable_contract() {
    for code in [
        "COMMIT_PREPARATION_BUSY",
        "COMMIT_PREPARATION_STALE",
        "PREPARATION_STALE",
    ] {
        let mapped = error(storage_engine::Error::new(code, "retry preparation").retryable());
        assert_eq!(mapped.status, 409);
        assert_eq!(mapped.code, code);
        assert!(mapped.retryable);
    }
}

#[test]
fn upload_generation_and_lease_errors_preserve_conflict_status() {
    for (core_code, api_code) in [
        ("STALE_UPLOAD_GENERATION", "UPLOAD_GENERATION_CONFLICT"),
        ("UPLOAD_GENERATION_CONFLICT", "UPLOAD_GENERATION_CONFLICT"),
        ("FENCING_TOKEN_EXPIRED", "LEASE_EXPIRED"),
    ] {
        let error = error(storage_engine::Error::new(
            core_code,
            "retry needs new state",
        ));
        assert_eq!(error.status, 409, "{core_code}");
        assert_eq!(error.code, api_code);
    }
}

#[test]
fn transport_acl_matches_core_capabilities_and_preserves_account_admission() {
    let mut repository = Repository {
        repository_id: "repository".into(),
        name: "Repository".into(),
        history_tree_id: "history".into(),
        main_branch_id: "branch".into(),
        primary_slot_id: "P01".into(),
        target_copies: 3,
        name_policy: "PortableName".into(),
        unicode_version: "16.0.0".into(),
        grants: Grants::default(),
        account_scope_repository_id: None,
        smb_enabled: false,
        smb_binding_generation: 1,
        file_manager_root: false,
    };
    let mut identity = Identity {
        admission_session: None,
        smb_binding: None,
        principal: "user".into(),
        admin: false,
        repositories: BTreeSet::from([repository.repository_id.clone()]),
    };
    for bits in 0..8 {
        repository.grants = Grants::default();
        if bits & 1 != 0 {
            repository.grants.read.insert("user".into());
        }
        if bits & 2 != 0 {
            repository.grants.write.insert("user".into());
        }
        if bits & 4 != 0 {
            repository.grants.manage.insert("user".into());
        }
        for access in [
            RepositoryAccess::Read,
            RepositoryAccess::Write,
            RepositoryAccess::Manage,
        ] {
            assert_eq!(
                authorize_access(&identity, &repository, access).is_ok(),
                repository.grants.allows("user", access)
            );
            identity.repositories.clear();
            assert!(authorize_access(&identity, &repository, access).is_err());
            identity.admin = true;
            assert!(authorize_access(&identity, &repository, access).is_ok());
            identity.admin = false;
            identity
                .repositories
                .insert(repository.repository_id.clone());
        }
    }
}

#[test]
fn split_repository_accepts_independent_account_admission_and_still_requires_its_own_acl() {
    let mut repository = Repository {
        repository_id: "new repository".into(),
        name: "Independent split".into(),
        history_tree_id: "history".into(),
        main_branch_id: "branch".into(),
        primary_slot_id: "P01".into(),
        target_copies: 3,
        name_policy: "PortableName".into(),
        unicode_version: "16.0.0".into(),
        grants: Grants::owner("reader"),
        account_scope_repository_id: Some("source repository".into()),
        smb_enabled: false,
        smb_binding_generation: 1,
        file_manager_root: false,
    };
    let direct = Identity {
        admission_session: None,
        smb_binding: None,
        principal: "reader".into(),
        admin: false,
        repositories: BTreeSet::from([repository.repository_id.clone()]),
    };
    let inherited = Identity {
        admission_session: None,
        smb_binding: None,
        repositories: BTreeSet::from(["source repository".into()]),
        ..direct.clone()
    };
    assert!(authorize_access(&direct, &repository, RepositoryAccess::Read).is_ok());
    assert!(authorize_access(&inherited, &repository, RepositoryAccess::Read).is_ok());
    let unrelated = Identity {
        admission_session: None,
        smb_binding: None,
        repositories: BTreeSet::from(["unrelated repository".into()]),
        ..direct.clone()
    };
    assert!(authorize_access(&unrelated, &repository, RepositoryAccess::Read).is_err());
    repository.grants = Grants::default();
    assert!(authorize_access(&direct, &repository, RepositoryAccess::Read).is_err());
    assert!(authorize_access(&inherited, &repository, RepositoryAccess::Read).is_err());
}
