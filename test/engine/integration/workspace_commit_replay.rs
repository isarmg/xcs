use std::fs;
use storage_engine::{CommitRequest, Config, Engine, new_id};

fn saved() -> (tempfile::TempDir, Engine, String, CommitRequest) {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::development([
        temp.path().join("primary"),
        temp.path().join("backup"),
        temp.path().join("tertiary"),
    ]);
    let mut engine = Engine::initialize(&temp.path().join("system"), config).unwrap();
    let repo = engine
        .create_repository("files", "P01", 1, "admin", &new_id("op"))
        .unwrap();
    let mut grants = repo.grants.clone();
    grants.write.insert("writer".into());
    engine
        .set_grants(&repo.repository_id, grants, "admin", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, "writer")
        .unwrap();
    engine
        .stage_bytes(&workspace.workspace_id, "file", b"retained history")
        .unwrap();
    let request = CommitRequest {
        workspace_id: workspace.workspace_id,
        operation_id: new_id("op"),
        principal: "writer".into(),
        expected_head: workspace.base_head,
        fencing_token: workspace.fencing_token,
        message: "original save".into(),
        request_digest: None,
    };
    engine.commit_workspace(request.clone()).unwrap();
    (temp, engine, repo.repository_id, request)
}

#[test]
fn native_commit_replay_rechecks_current_permission() {
    let (_temp, mut engine, repository, request) = saved();
    let mut grants = engine.state().repositories[&repository].grants.clone();
    grants.write.remove("writer");
    engine
        .set_grants(&repository, grants, "admin", &new_id("op"))
        .unwrap();
    assert_eq!(
        engine.commit_workspace(request).unwrap_err().code(),
        "PERMISSION_DENIED"
    );
}

#[test]
fn native_commit_replay_survives_system_disk_restore() {
    let (temp, mut engine, _repository, request) = saved();
    let original = engine.commit_workspace(request.clone()).unwrap();
    let disks = engine
        .config()
        .disks
        .iter()
        .map(|d| d.root.clone())
        .collect::<Vec<_>>();
    drop(engine);
    fs::remove_dir_all(temp.path().join("system")).unwrap();
    let mut engine = Engine::restore_from_disks(&temp.path().join("replacement"), &disks).unwrap();
    assert_eq!(engine.commit_workspace(request.clone()).unwrap(), original);
    let prepared = engine
        .prepare_workspace_commit(request, None)
        .unwrap()
        .execute()
        .unwrap();
    assert_eq!(engine.finish_workspace_commit(prepared).unwrap(), original);
}

fn request_paths(engine: &Engine, request: &CommitRequest) -> [std::path::PathBuf; 2] {
    [
        engine
            .root()
            .join("workspaces")
            .join(&request.workspace_id)
            .join("commit-request.json"),
        engine.config().disks[0]
            .root
            .join("staging/workspaces")
            .join(&request.workspace_id)
            .join("commit-request.json"),
    ]
}

#[test]
fn native_commit_replay_uses_only_exact_bound_request_copies() {
    for corrupt in [false, true] {
        let (_temp, mut engine, _repository, request) = saved();
        let paths = request_paths(&engine, &request);
        let original = engine.commit_workspace(request.clone()).unwrap();
        let bad = if corrupt {
            b"corrupt".to_vec()
        } else {
            let mut other = request.clone();
            other.message = "different request".into();
            serde_json::to_vec(&other).unwrap()
        };
        fs::write(&paths[0], &bad).unwrap();
        assert_eq!(engine.commit_workspace(request.clone()).unwrap(), original);
        fs::write(&paths[1], &bad).unwrap();
        assert_eq!(
            engine.commit_workspace(request.clone()).unwrap_err().code(),
            "RECOVERY_REQUIRED"
        );
        assert_eq!(fs::read(&paths[0]).unwrap(), bad);
        assert_eq!(fs::read(&paths[1]).unwrap(), bad);
    }
}

#[test]
fn native_commit_replay_does_not_trust_matching_but_unbound_saved_json() {
    let (_temp, mut engine, _repository, mut request) = saved();
    let paths = request_paths(&engine, &request);
    request.message = "not the published request".into();
    let forged = serde_json::to_vec(&request).unwrap();
    for path in paths {
        fs::write(path, &forged).unwrap();
    }
    assert_eq!(
        engine.commit_workspace(request).unwrap_err().code(),
        "IDEMPOTENCY_KEY_REUSED"
    );
}

#[test]
fn unrelated_saved_request_cannot_close_an_unsubmitted_workspace() {
    let (_temp, mut engine, repository, request) = saved();
    let branch = engine.state().repositories[&repository]
        .main_branch_id
        .clone();
    let pending = engine.create_workspace(&branch, "writer").unwrap();
    engine
        .stage_bytes(&pending.workspace_id, "pending", b"not committed")
        .unwrap();
    let mut target = request.clone();
    target.workspace_id = pending.workspace_id.clone();
    for path in request_paths(&engine, &target) {
        fs::write(path, serde_json::to_vec(&request).unwrap()).unwrap();
    }
    let root = engine.root().to_owned();
    drop(engine);
    let engine = Engine::open(&root).unwrap();
    assert_eq!(
        engine.workspace(&pending.workspace_id).unwrap().state,
        "open"
    );
}

#[test]
fn forged_saved_request_identity_cannot_close_unsubmitted_bytes() {
    let (_temp, mut engine, repository, mut request) = saved();
    let branch = engine.state().repositories[&repository]
        .main_branch_id
        .clone();
    let pending = engine.create_workspace(&branch, "writer").unwrap();
    engine
        .stage_bytes(&pending.workspace_id, "pending", b"not committed")
        .unwrap();
    request.workspace_id = pending.workspace_id.clone();
    request.expected_head = pending.base_head.clone();
    request.fencing_token = pending.fencing_token;
    // Copying a published digest does not turn another manifest into its input.
    request.request_digest = Some(
        engine
            .operation(&request.operation_id)
            .unwrap()
            .request_digest,
    );
    for path in request_paths(&engine, &request) {
        fs::write(path, serde_json::to_vec(&request).unwrap()).unwrap();
    }
    let root = engine.root().to_owned();
    drop(engine);
    let engine = Engine::open(&root).unwrap();
    assert_eq!(
        engine.workspace(&pending.workspace_id).unwrap().state,
        "open"
    );
}

#[test]
fn wire_commit_reconciles_after_publication_and_system_disk_loss() {
    for missing_legacy_body in [false, true] {
        let (temp, mut engine, repository, _request) = saved();
        let branch = engine.state().repositories[&repository]
            .main_branch_id
            .clone();
        let workspace = engine.create_workspace(&branch, "writer").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "wire", b"wire bytes")
            .unwrap();
        let mut body = engine
            .seal_workspace(&workspace.workspace_id, "writer")
            .unwrap();
        body["request_digest"] = serde_json::json!(
            storage_engine::commit_request_digest(&workspace.workspace_id, &body).unwrap()
        );
        let request = CommitRequest {
            workspace_id: workspace.workspace_id.clone(),
            operation_id: body["operation_id"].as_str().unwrap().into(),
            principal: "writer".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: String::new(),
            request_digest: None,
        };
        engine.inject_fault_once("after_publication_3");
        assert_eq!(
            engine
                .commit_workspace_wire(request.clone(), body.clone())
                .unwrap_err()
                .code(),
            "IN_DOUBT"
        );
        let disks = engine
            .config()
            .disks
            .iter()
            .map(|d| d.root.clone())
            .collect::<Vec<_>>();
        if missing_legacy_body {
            // Older writes kept wire bodies only on the lost system disk.
            fs::remove_file(
                request_paths(&engine, &request)[1].with_file_name("wire-request.json"),
            )
            .unwrap();
        }
        drop(engine);
        fs::remove_dir_all(temp.path().join("system")).unwrap();
        let mut engine =
            Engine::restore_from_disks(&temp.path().join("replacement"), &disks).unwrap();
        assert_eq!(
            engine.workspace(&workspace.workspace_id).unwrap().state,
            if missing_legacy_body {
                "sealed"
            } else {
                "committed"
            }
        );
        let sequence = engine.state().sequence;
        let result = engine.commit_workspace_wire(request, body).unwrap();
        assert_eq!(
            result.commit_state,
            storage_contract::CommitState::Published
        );
        assert_eq!(engine.state().sequence, sequence);
        let retained = engine.workspace(&workspace.workspace_id).unwrap();
        assert_eq!(
            fs::read(&retained.files.values().next().unwrap().source_path).unwrap(),
            b"wire bytes"
        );
    }
}

#[test]
fn native_commit_replay_never_follows_unsafe_request_copy() {
    let (temp, mut engine, _repository, request) = saved();
    let [local, _data] = request_paths(&engine, &request);
    let external = temp.path().join("external");
    fs::write(&external, b"untouched").unwrap();
    fs::remove_file(&local).unwrap();
    std::os::unix::fs::symlink(&external, &local).unwrap();
    assert_eq!(
        engine.commit_workspace(request).unwrap_err().code(),
        "UNSAFE_PATH"
    );
    assert_eq!(fs::read(external).unwrap(), b"untouched");
}

fn saved_wire() -> (tempfile::TempDir, Engine, CommitRequest, serde_json::Value) {
    let (temp, mut engine, repository, _) = saved();
    let branch = engine.state().repositories[&repository]
        .main_branch_id
        .clone();
    let workspace = engine.create_workspace(&branch, "writer").unwrap();
    engine
        .stage_bytes(&workspace.workspace_id, "wire", b"wire bytes")
        .unwrap();
    let mut body = engine
        .seal_workspace(&workspace.workspace_id, "writer")
        .unwrap();
    body["request_digest"] = serde_json::json!(
        storage_engine::commit_request_digest(&workspace.workspace_id, &body).unwrap()
    );
    let request = CommitRequest {
        workspace_id: workspace.workspace_id,
        operation_id: body["operation_id"].as_str().unwrap().into(),
        principal: "writer".into(),
        expected_head: workspace.base_head,
        fencing_token: workspace.fencing_token,
        message: String::new(),
        request_digest: None,
    };
    engine
        .commit_workspace_wire(request.clone(), body.clone())
        .unwrap();
    (temp, engine, request, body)
}

#[test]
fn wire_receipt_cannot_close_a_different_manifest_with_the_same_workspace_id() {
    for namespace in [false, true] {
        let (_temp, engine, request, _) = saved_wire();
        let root = engine.root().to_owned();
        drop(engine);
        // A completed workspace has advanced fencing, but its unchanged sealed
        // manifest must still reconcile normally using the saved request fence.
        let engine = Engine::open(&root).unwrap();
        let mut workspace = engine.workspace(&request.workspace_id).unwrap();
        assert_eq!(workspace.state, "committed");
        assert_eq!(workspace.fencing_token, request.fencing_token + 1);
        workspace.state = "sealed".into();
        if namespace {
            workspace
                .namespace_operations
                .push(storage_engine::NamespaceOperation {
                    kind: "mkdir".into(),
                    path: "not-published".into(),
                    destination: None,
                });
        } else {
            let staged = workspace.files.values_mut().next().unwrap();
            staged.size += 1;
            staged.content_hash = "0".repeat(64);
        }
        for path in request_paths(&engine, &request) {
            fs::write(
                path.with_file_name("workspace.json"),
                serde_json::to_vec(&workspace).unwrap(),
            )
            .unwrap();
        }
        drop(engine);
        let engine = Engine::open(&root).unwrap();
        assert_eq!(
            engine.workspace(&request.workspace_id).unwrap().state,
            "sealed"
        );
        assert_eq!(
            engine
                .operation(&request.operation_id)
                .unwrap()
                .commit_state,
            storage_contract::CommitState::Published
        );
    }
}

fn assert_wire_request_binding(detached: bool) {
    for change_fence in [false, true] {
        let (_temp, mut engine, mut request, body) = saved_wire();
        if change_fence {
            request.fencing_token += 1;
        } else {
            request.message = "different".into();
        }
        let error = if detached {
            engine
                .prepare_workspace_commit(request, Some(body))
                .unwrap()
                .execute()
                .err()
                .expect("changed request must not replay")
        } else {
            engine.commit_workspace_wire(request, body).unwrap_err()
        };
        assert_eq!(error.code(), "REQUEST_BINDING_MISMATCH");
    }
}

#[test]
fn wire_replay_checks_request_message_and_fencing_against_body() {
    assert_wire_request_binding(false);
}

#[test]
fn detached_wire_replay_checks_request_message_and_fencing_against_body() {
    assert_wire_request_binding(true);
}
