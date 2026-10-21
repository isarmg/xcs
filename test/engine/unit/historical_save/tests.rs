use super::*;
#[test]
fn historical_edits_materialize_only_written_files_without_changing_the_source_head() {
    let root = tempfile::tempdir().unwrap();
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
    let repo = engine
        .create_repository("history", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, "admin")
        .unwrap();
    engine
        .stage_bytes(
            &workspace.workspace_id,
            "unchanged.txt",
            b"independent source",
        )
        .unwrap();
    engine
        .stage_bytes(&workspace.workspace_id, "edited.txt", b"old complete file")
        .unwrap();
    let saved = engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "admin".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "Original".into(),
            request_digest: None,
        })
        .unwrap();
    let source = saved.commit_id.unwrap();
    let historical = engine
        .create_historical_workspace(&repo.main_branch_id, &source, "admin")
        .unwrap();
    engine
        .stage_bytes(
            &historical.workspace_id,
            "edited.txt",
            b"historical replacement",
        )
        .unwrap();
    let ordinary = engine.commit_workspace(CommitRequest {
        workspace_id: historical.workspace_id.clone(),
        operation_id: new_id("op"),
        principal: "admin".into(),
        expected_head: historical.base_head.clone(),
        fencing_token: historical.fencing_token,
        message: "must refuse".into(),
        request_digest: None,
    });
    assert!(ordinary.is_err());
    let plan = engine
        .create_historical_save_plan(
            &repo.history_tree_id,
            HistoricalSaveOptions {
                source_commit_id: source.clone(),
                name: "from history".into(),
                workspace_id: Some(historical.workspace_id),
                primary_slot_id: None,
                target_copies: None,
            },
            "admin",
        )
        .unwrap();
    let sequence = engine.state.sequence;
    let operation = new_id("op");
    let saved = engine
        .commit_historical_save_plan(&plan.plan_id, "admin", &operation)
        .unwrap();
    assert_eq!(engine.state.sequence, sequence + 1);
    assert_eq!(
        engine.state.branches[&repo.main_branch_id].head_commit_id,
        source
    );
    let branch = saved.branch_id.clone().unwrap();
    let entries = engine.list_tree(&branch).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        saved.result["version_protection"].as_array().unwrap().len(),
        1
    );
    let old = engine.state.commits[&source]
        .entries
        .iter()
        .find(|entry| entry.path == "unchanged.txt")
        .unwrap();
    let new = entries
        .iter()
        .find(|entry| entry.path == "unchanged.txt")
        .unwrap();
    assert_eq!(new.version_id, old.version_id);
    assert_eq!(new.size, old.size);
    assert_eq!(
        engine.pending_file_source(&branch, &new.file_id),
        Some(old.version_id.as_str())
    );
    assert_eq!(saved.result["written_files"], 1);
    assert_eq!(saved.result["placeholder_files"], 1);
    let edited = entries
        .iter()
        .find(|entry| entry.path == "edited.txt")
        .unwrap();
    let mut content = engine.open_content(&edited.version_id).unwrap();
    let mut bytes = Vec::new();
    use std::io::Read;
    content.file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"historical replacement");
    drop(content);
    assert_eq!(
        engine
            .commit_historical_save_plan(&plan.plan_id, "admin", &operation)
            .unwrap()
            .commit_id,
        saved.commit_id
    );
}

#[test]
fn historical_placeholders_need_no_old_payload_and_written_replacements_materialize() {
    let root = tempfile::tempdir().unwrap();
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
    let repo = engine
        .create_repository("purged history", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let mut commits = Vec::new();
    for bytes in [b"old full file".as_slice(), b"current full file".as_slice()] {
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "file.txt", bytes)
            .unwrap();
        let operation = engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "Save".into(),
                request_digest: None,
            })
            .unwrap();
        commits.push(operation.commit_id.unwrap());
    }
    let old_version = engine.state.commits[&commits[0]].entries[0]
        .version_id
        .clone();
    let purge = engine
        .create_purge_plan(&repo.history_tree_id, vec![old_version], "admin")
        .unwrap();
    engine
        .commit_plan(&purge.plan_id, "admin", &new_id("op"))
        .unwrap();
    let options = HistoricalSaveOptions {
        source_commit_id: commits[0].clone(),
        name: "requires missing bytes".into(),
        workspace_id: None,
        primary_slot_id: None,
        target_copies: None,
    };
    let version_count = engine.state.versions.len();
    let placeholder_plan = engine
        .create_historical_save_plan(&repo.history_tree_id, options, "admin")
        .unwrap();
    assert_eq!(placeholder_plan.input["full_copy_files"], 0);
    assert_eq!(placeholder_plan.input["placeholder_files"], 1);
    let placeholder = engine
        .commit_historical_save_plan(&placeholder_plan.plan_id, "admin", &new_id("op"))
        .unwrap();
    assert_eq!(engine.state.versions.len(), version_count);
    let inherited = engine
        .list_tree(placeholder.branch_id.as_deref().unwrap())
        .unwrap();
    assert_eq!(
        inherited[0].version_id,
        engine.state.commits[&commits[0]].entries[0].version_id
    );
    assert_eq!(
        engine
            .open_content(&inherited[0].version_id)
            .err()
            .unwrap()
            .code(),
        "CONTENT_PURGED"
    );
    let workspace = engine
        .create_historical_workspace(&repo.main_branch_id, &commits[0], "admin")
        .unwrap();
    engine
        .stage_bytes(
            &workspace.workspace_id,
            "file.txt",
            b"explicit complete replacement",
        )
        .unwrap();
    let plan = engine
        .create_historical_save_plan(
            &repo.history_tree_id,
            HistoricalSaveOptions {
                source_commit_id: commits[0].clone(),
                name: "explicit historical replacement".into(),
                workspace_id: Some(workspace.workspace_id),
                primary_slot_id: None,
                target_copies: None,
            },
            "admin",
        )
        .unwrap();
    let saved = engine
        .commit_plan(&plan.plan_id, "admin", &new_id("op"))
        .unwrap();
    assert_eq!(
        engine.state.branches[&repo.main_branch_id].head_commit_id,
        commits[1]
    );
    assert!(saved.branch_id.is_some());
}
