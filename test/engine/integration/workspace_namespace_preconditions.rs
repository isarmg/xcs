//! A workspace acts on the stable entries it opened, not later namesakes.
use storage_engine::{CommitRequest, Config, Engine, NamespaceOperation, new_id};

#[test]
fn stale_namespace_operations_preserve_recreated_same_name_directories() {
    for kind in ["delete", "delete_tree", "rename"] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("system");
        let config = Config::development([
            temporary.path().join("primary"),
            temporary.path().join("backup"),
            temporary.path().join("tertiary"),
        ]);
        let mut engine = Engine::initialize(&root, config).unwrap();
        let repo = engine
            .create_repository("Namespace preconditions", "P01", 3, "admin", &new_id("op"))
            .unwrap();
        let change = |kind: &str| NamespaceOperation {
            kind: kind.into(),
            path: "Documents".into(),
            destination: (kind == "rename").then(|| "Moved".into()),
        };
        engine
            .file_namespace(&repo.repository_id, change("mkdir"), "admin", &new_id("op"))
            .unwrap();
        let original = engine.list_tree(&repo.main_branch_id).unwrap()[0].clone();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        engine
            .namespace_operation(&workspace.workspace_id, change(kind))
            .unwrap();
        engine
            .file_namespace(
                &repo.repository_id,
                change("delete"),
                "admin",
                &new_id("op"),
            )
            .unwrap();
        engine
            .file_namespace(&repo.repository_id, change("mkdir"), "admin", &new_id("op"))
            .unwrap();
        let replacement = engine.list_tree(&repo.main_branch_id).unwrap()[0].clone();
        assert_ne!(original.entry_id, replacement.entry_id);
        // Preconditions must survive service restarts alongside the pending work.
        drop(engine);
        let mut engine = Engine::open(&root).unwrap();
        let error = engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id.clone(),
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "Apply pending namespace change".into(),
                request_digest: None,
            })
            .unwrap_err();
        assert_eq!(error.code(), "NAMESPACE_CONFLICT", "{kind}");
        let current = engine.list_tree(&repo.main_branch_id).unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].entry_id, replacement.entry_id);
        assert_eq!(current[0].path, "Documents");
        assert_eq!(
            engine.workspace(&workspace.workspace_id).unwrap().state,
            "open"
        );
    }
}
