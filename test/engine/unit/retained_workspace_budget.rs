use super::*;

#[test]
fn aborted_records_consume_global_and_principal_budgets_after_restart() {
    let mut f = Fixture::new();
    f.config.max_retained_workspaces = 3;
    f.config.max_retained_workspaces_per_principal = 2;
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    let mut grants = repo.grants.clone();
    grants.write.insert("writer".into());
    e.set_grants(&repo.repository_id, grants, "admin", &new_id("op"))
        .unwrap();
    for _ in 0..2 {
        let w = e.create_workspace(&repo.main_branch_id, "writer").unwrap();
        e.abort_workspace(&w.workspace_id, "writer").unwrap();
    }
    e.assert_workspace_budget_consistent();
    drop(e);
    let mut e = Engine::open(&f.root).unwrap();
    assert_eq!(e.workspaces.len(), 2);
    assert_eq!(
        e.create_workspace(&repo.main_branch_id, "writer")
            .unwrap_err()
            .code(),
        "RESOURCE_LIMIT"
    );
    let w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
    e.abort_workspace(&w.workspace_id, "admin").unwrap();
    assert_eq!(
        e.create_workspace(&repo.main_branch_id, "admin")
            .unwrap_err()
            .code(),
        "RESOURCE_LIMIT"
    );
    e.assert_workspace_budget_consistent();
    drop(e);
    let mut e = Engine::open(&f.root).unwrap();
    assert_eq!(e.workspaces.len(), 3);
    assert_eq!(
        e.create_workspace(&repo.main_branch_id, "admin")
            .unwrap_err()
            .code(),
        "RESOURCE_LIMIT"
    );
}

#[test]
fn namespace_growth_is_charged_and_rejection_preserves_durable_evidence() {
    for principal_limit in [false, true] {
        let mut f = Fixture::new();
        if principal_limit {
            f.config.max_retained_workspace_metadata_bytes_per_principal = 2048;
        } else {
            f.config.max_retained_workspace_metadata_bytes = 2048;
        }
        let mut e = f.initialize();
        let repo = repository(&mut e, 1);
        let w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
        let mut accepted = 0;
        loop {
            let before = canonical_json(&e.workspace(&w.workspace_id).unwrap()).unwrap();
            match e.namespace_operation(
                &w.workspace_id,
                NamespaceOperation {
                    kind: "mkdir".into(),
                    path: format!("directory-{accepted:04}"),
                    destination: None,
                },
            ) {
                Ok(_) => {
                    accepted += 1;
                    assert!(accepted < 100, "manifest admission must be bounded");
                }
                Err(error) => {
                    assert_eq!(error.code(), "RESOURCE_LIMIT");
                    assert_eq!(
                        canonical_json(&e.workspace(&w.workspace_id).unwrap()).unwrap(),
                        before
                    );
                    break;
                }
            }
        }
        assert!(accepted > 0);
        // Reserved lifecycle headroom permits closure even at the byte ceiling.
        e.abort_workspace(&w.workspace_id, "admin").unwrap();
        e.assert_workspace_budget_consistent();
        drop(e);
        let mut e = Engine::open(&f.root).unwrap();
        let saved = e.workspace(&w.workspace_id).unwrap();
        assert_eq!(saved.state, "aborted");
        assert_eq!(saved.namespace_operations.len(), accepted);
        assert_eq!(
            e.create_workspace(&repo.main_branch_id, "admin")
                .unwrap_err()
                .code(),
            "RESOURCE_LIMIT"
        );
    }
}

#[test]
fn uncertain_manifest_write_blocks_further_admission_until_restart() {
    let f = Fixture::new();
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    let w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
    let manifest = f.config.disks[0]
        .root
        .join("staging/workspaces")
        .join(&w.workspace_id)
        .join("workspace.json");
    let bytes = fs::read(&manifest).unwrap();
    fs::remove_file(&manifest).unwrap();
    fs::create_dir(&manifest).unwrap();
    assert!(
        e.namespace_operation(
            &w.workspace_id,
            NamespaceOperation {
                kind: "mkdir".into(),
                path: "unacknowledged".into(),
                destination: None,
            }
        )
        .is_err()
    );
    assert!(
        e.workspace(&w.workspace_id)
            .unwrap()
            .namespace_operations
            .is_empty()
    );
    assert!(e.create_workspace(&repo.main_branch_id, "admin").is_err());
    assert!(e.blocked);
    fs::remove_dir(&manifest).unwrap();
    fs::write(&manifest, bytes).unwrap();
    e.assert_workspace_budget_consistent();
    drop(e);
    let e = Engine::open(&f.root).unwrap();
    assert!(
        e.workspace(&w.workspace_id)
            .unwrap()
            .namespace_operations
            .is_empty()
    );
}

#[test]
fn over_budget_restart_fails_without_discarding_records() {
    let f = Fixture::new();
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    for _ in 0..2 {
        let w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
        e.abort_workspace(&w.workspace_id, "admin").unwrap();
    }
    e.assert_workspace_budget_consistent();
    drop(e);
    let mut smaller = f.config.clone();
    smaller.max_retained_workspaces = 1;
    crate::disk::write_json(&f.root.join("config.json"), &smaller, true).unwrap();
    let error = match Engine::open(&f.root) {
        Ok(_) => panic!("startup must reject over-budget retained evidence"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "RESOURCE_LIMIT");
    crate::disk::write_json(&f.root.join("config.json"), &f.config, true).unwrap();
    assert_eq!(Engine::open(&f.root).unwrap().workspaces.len(), 2);
}

#[test]
fn rejected_initial_manifest_does_not_consume_durable_record_capacity() {
    let mut f = Fixture::new();
    f.config.max_retained_workspace_metadata_bytes = 1;
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    for _ in 0..3 {
        assert_eq!(
            e.create_workspace(&repo.main_branch_id, "admin")
                .unwrap_err()
                .code(),
            "RESOURCE_LIMIT"
        );
        assert!(e.workspaces.is_empty());
    }
    for root in [
        f.root.join("workspaces"),
        f.config.disks[0].root.join("staging/workspaces"),
    ] {
        if root.exists() {
            assert_eq!(fs::read_dir(root).unwrap().count(), 0);
        }
    }
    e.assert_workspace_budget_consistent();
    drop(e);
    assert!(Engine::open(&f.root).unwrap().workspaces.is_empty());
}

#[test]
fn committed_file_context_receipt_fits_exact_reserved_metadata_budget() {
    let f = Fixture::new();
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    save(&mut e, &repo.main_branch_id, "existing", b"earlier payload");
    let w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
    let old_contexts = w.selected_file_contexts.clone();
    assert!(!old_contexts.is_empty());
    e.namespace_operation(
        &w.workspace_id,
        NamespaceOperation {
            kind: "mkdir".into(),
            path: "extra-directory".into(),
            destination: None,
        },
    )
    .unwrap();
    e.stage_bytes(&w.workspace_id, "nested/new-file", b"retained payload")
        .unwrap();
    let charge = e.retained_workspace_metadata_bytes();
    e.config.max_retained_workspace_metadata_bytes = charge;
    e.config.max_retained_workspace_metadata_bytes_per_principal = charge;
    crate::disk::write_json(&f.root.join("config.json"), &e.config, true).unwrap();
    e.commit_workspace(CommitRequest {
        workspace_id: w.workspace_id.clone(),
        operation_id: new_id("op"),
        principal: "admin".into(),
        expected_head: w.base_head,
        fencing_token: w.fencing_token,
        message: "exact metadata budget".into(),
        request_digest: None,
    })
    .unwrap();
    assert_eq!(e.workspace(&w.workspace_id).unwrap().state, "committed");
    assert_eq!(e.retained_workspace_metadata_bytes(), charge);
    let completed = e.workspace(&w.workspace_id).unwrap();
    for (key, context) in &old_contexts {
        assert_eq!(completed.selected_file_contexts.get(key), Some(context));
    }
    assert_eq!(completed.namespace_operations.len(), 1);
    e.assert_workspace_budget_consistent();
    drop(e);
    let e = Engine::open(&f.root).unwrap();
    assert_eq!(e.workspace(&w.workspace_id).unwrap().state, "committed");
    assert_eq!(e.retained_workspace_metadata_bytes(), charge);
    e.assert_workspace_budget_consistent();
}

#[test]
fn retained_record_owner_cannot_move_its_charge_to_another_principal() {
    let f = Fixture::new();
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    let mut w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
    let charge = e.retained_workspace_metadata_bytes();
    w.principal = "another-writer".into();
    assert_eq!(
        e.check_workspace_budget(&w).map(|_| ()).unwrap_err().code(),
        "STATE_INVALID"
    );
    assert_eq!(e.retained_workspace_metadata_bytes(), charge);
    e.assert_workspace_budget_consistent();
}

#[test]
fn zero_byte_generations_consume_retained_entry_budget_without_extra_artifacts() {
    let mut f = Fixture::new();
    f.config.max_entries = 2;
    let mut e = f.initialize();
    let repo = repository(&mut e, 1);
    let w = e.create_workspace(&repo.main_branch_id, "admin").unwrap();
    let source = f.temp.path().join("empty");
    fs::write(&source, b"").unwrap();
    let item = new_id("entry");
    let first = e
        .stage_upload(&w.workspace_id, &item, "empty", &source, 1)
        .unwrap();
    let second = e
        .stage_upload(&w.workspace_id, &item, "empty", &source, 2)
        .unwrap();
    let directory = first.source_path.parent().unwrap();
    let names = || {
        let mut entries: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        entries.sort();
        entries
    };
    let before = names();
    for _ in 0..3 {
        assert_eq!(
            e.stage_upload(&w.workspace_id, &item, "empty", &source, 3)
                .unwrap_err()
                .code(),
            "RESOURCE_LIMIT"
        );
        assert_eq!(
            names(),
            before,
            "rejected generation must not create a scratch artifact"
        );
    }
    assert_eq!(fs::metadata(&first.source_path).unwrap().len(), 0);
    assert_eq!(fs::metadata(&second.source_path).unwrap().len(), 0);
    e.stage_upload(&w.workspace_id, &item, "empty", &source, 2)
        .unwrap();
    e.assert_workspace_budget_consistent();
    drop(e);
    let mut e = Engine::open(&f.root).unwrap();
    assert_eq!(
        e.stage_upload(&w.workspace_id, &item, "empty", &source, 3)
            .unwrap_err()
            .code(),
        "RESOURCE_LIMIT"
    );
    assert_eq!(names(), before);
}
