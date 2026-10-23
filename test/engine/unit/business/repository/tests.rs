use super::*;

#[test]
fn disk_root_uses_one_explicit_primary_on_short_and_complete_routes() {
    for disks in [1, 2, 3] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("system");
        let mut config = Config::development([
            temporary.path().join("primary"),
            temporary.path().join("backup"),
            temporary.path().join("tertiary"),
        ]);
        config.disks.truncate(disks);
        let mut engine = Engine::initialize(&root, config).unwrap();
        let repo = engine
            .create_disk_root("p01", "admin", &new_id("op"))
            .unwrap();
        assert!(repo.file_manager_root);
        assert_eq!(repo.primary_slot_id, "P01");
        assert_eq!(repo.target_copies, 1);
        assert_eq!(
            engine
                .create_disk_root("P01", "admin", &new_id("op"))
                .unwrap()
                .repository_id,
            repo.repository_id
        );
        engine
            .file_namespace(
                &repo.repository_id,
                NamespaceOperation {
                    kind: "mkdir".into(),
                    path: "Documents".into(),
                    destination: None,
                },
                "admin",
                &new_id("op"),
            )
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        engine
            .stage_bytes(
                &workspace.workspace_id,
                "Documents/report.txt",
                b"one primary",
            )
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "Save on configured primary".into(),
                request_digest: None,
            })
            .unwrap();
        let entry = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        let placement = &engine.state().placements[&entry.version_id];
        assert_eq!(placement.target_copies, 1);
        assert_eq!(placement.route_slot_ids, vec!["P01".to_owned()]);
        drop(engine);
        let engine = Engine::open(&root).unwrap();
        let mut content = engine.open_content(&entry.version_id).unwrap();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut content.file, &mut bytes).unwrap();
        assert_eq!(bytes, b"one primary");
    }
}

#[test]
fn explicit_smb_binding_changes_advance_a_persistent_independent_revision() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("system");
    let mut engine = Engine::initialize(
        &root,
        Config::development([
            temporary.path().join("primary"),
            temporary.path().join("backup"),
            temporary.path().join("tertiary"),
        ]),
    )
    .unwrap();
    let repo = engine
        .create_disk_root("P01", "admin", &new_id("op"))
        .unwrap();
    assert_eq!(repo.smb_binding_generation, 1);
    for (index, enabled) in [true, false, true].into_iter().enumerate() {
        let expected = engine.state().histories[&repo.history_tree_id].generation;
        let operation = new_id("op");
        let first = engine
            .set_smb_binding(
                &repo.repository_id,
                &repo.main_branch_id,
                enabled,
                expected,
                "admin",
                &operation,
            )
            .unwrap();
        assert_eq!(
            engine.state().repositories[&repo.repository_id].smb_binding_generation,
            index as u64 + 2
        );
        let replay = engine
            .set_smb_binding(
                &repo.repository_id,
                &repo.main_branch_id,
                enabled,
                expected,
                "admin",
                &operation,
            )
            .unwrap();
        assert_eq!(first.result, replay.result);
        assert_eq!(
            engine.state().repositories[&repo.repository_id].smb_binding_generation,
            index as u64 + 2
        );
    }
    drop(engine);
    let engine = Engine::open(&root).unwrap();
    assert_eq!(
        engine.state().repositories[&repo.repository_id].smb_binding_generation,
        4
    );
}
