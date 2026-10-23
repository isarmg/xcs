use super::*;

#[test]
fn detached_snapshot_keeps_only_target_and_preserves_live_retained_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("state");
    let config = Config::development([temp.path().join("primary")]);
    let mut engine = Engine::initialize(&root, config).unwrap();
    let repo = engine
        .create_repository("planning", "P01", 1, "alice", &new_id("op"))
        .unwrap();
    let mut grants = repo.grants.clone();
    grants.write.insert("bob".into());
    engine
        .set_grants(&repo.repository_id, grants, "alice", &new_id("op"))
        .unwrap();
    for (principal, abort) in [("alice", true), ("bob", true), ("bob", false)] {
        let workspace = engine
            .create_workspace(&repo.main_branch_id, principal)
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "unrelated", b"retained payload")
            .unwrap();
        if abort {
            engine
                .abort_workspace(&workspace.workspace_id, principal)
                .unwrap();
        }
    }
    let unrelated = engine.workspaces.clone();
    let target = engine
        .create_workspace(&repo.main_branch_id, "alice")
        .unwrap();
    engine
        .stage_bytes(&target.workspace_id, "target", b"target payload")
        .unwrap();
    let request = CommitRequest {
        workspace_id: target.workspace_id.clone(),
        operation_id: new_id("op"),
        principal: "alice".into(),
        expected_head: target.base_head,
        fencing_token: target.fencing_token,
        message: "target-only snapshot".into(),
        request_digest: None,
    };
    let before = canonical_json(&engine.workspaces).unwrap();
    let capacities: Vec<_> = ["alice", "bob", "absent"]
        .into_iter()
        .map(|principal| (principal, engine.workspace_capacity(principal, true)))
        .collect();
    let preparation = engine
        .prepare_workspace_commit(request.clone(), None)
        .unwrap();
    let planning = &preparation.snapshot.engine;
    assert_eq!(planning.workspaces.len(), 1);
    assert_eq!(
        canonical_json(&planning.workspace(&target.workspace_id).unwrap()).unwrap(),
        canonical_json(&engine.workspace(&target.workspace_id).unwrap()).unwrap()
    );
    for workspace_id in unrelated.keys() {
        assert!(!planning.workspaces.contains_key(workspace_id));
    }
    for (principal, capacity) in &capacities {
        assert_eq!(&planning.workspace_capacity(principal, true), capacity);
        assert_eq!(&engine.workspace_capacity(principal, true), capacity);
    }
    assert_eq!(engine.workspaces.len(), 4);
    assert_eq!(canonical_json(&engine.workspaces).unwrap(), before);
    engine.assert_workspace_budget_consistent();
    let prepared = preparation.execute().unwrap();
    assert_eq!(canonical_json(&engine.workspaces).unwrap(), before);
    let result = engine.finish_workspace_commit(prepared).unwrap();
    assert_eq!(result.operation_id, request.operation_id);
    assert_eq!(
        engine.workspace(&target.workspace_id).unwrap().state,
        "committed"
    );
    assert_eq!(engine.workspaces.len(), 4);
    for (id, workspace) in &unrelated {
        assert_eq!(
            canonical_json(&engine.workspace(id).unwrap()).unwrap(),
            canonical_json(workspace).unwrap()
        );
    }
    engine.assert_workspace_budget_consistent();
    let committed = canonical_json(&engine.workspaces).unwrap();
    let capacities: Vec<_> = capacities
        .into_iter()
        .map(|(principal, _)| (principal, engine.workspace_capacity(principal, true)))
        .collect();
    drop(engine);
    let mut engine = Engine::open(&root).unwrap();
    assert_eq!(canonical_json(&engine.workspaces).unwrap(), committed);
    assert_eq!(engine.operation(&request.operation_id).unwrap(), result);
    let replay = engine
        .prepare_workspace_commit(request, None)
        .unwrap()
        .execute()
        .unwrap();
    assert_eq!(engine.finish_workspace_commit(replay).unwrap(), result);
    assert_eq!(canonical_json(&engine.workspaces).unwrap(), committed);
    for (principal, capacity) in capacities {
        assert_eq!(engine.workspace_capacity(principal, true), capacity);
    }
    engine.assert_workspace_budget_consistent();
}
