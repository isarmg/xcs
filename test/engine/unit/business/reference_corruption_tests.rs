//! Broken persisted references are recoverable errors, not mutex-poisoning panics.
use super::*;

fn fixture() -> (tempfile::TempDir, Engine, Repository, Entry, String) {
    let temporary = tempfile::tempdir().unwrap();
    let mut engine = Engine::initialize(
        &temporary.path().join("system"),
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
    let workspace = engine
        .create_workspace(&repo.main_branch_id, "admin")
        .unwrap();
    engine
        .stage_bytes(&workspace.workspace_id, "folder/file.txt", b"data")
        .unwrap();
    engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "admin".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "fixture".into(),
            request_digest: None,
        })
        .unwrap();
    let entry = engine.state.commits[&engine.state.branches[&repo.main_branch_id].head_commit_id]
        .entries
        .iter()
        .find(|entry| entry.kind == "file")
        .unwrap()
        .clone();
    let branch = engine
        .create_file_branch(
            &entry.file_id,
            &entry.version_id,
            "draft",
            "admin",
            &new_id("op"),
        )
        .unwrap()
        .result["branch_id"]
        .as_str()
        .unwrap()
        .to_owned();
    (temporary, engine, repo, entry, branch)
}

#[test]
fn file_selection_rejects_missing_heads_without_publication() {
    for missing in ["main_branch", "main_head", "file_head", "history"] {
        for scoped in [false, true] {
            let (_temporary, mut engine, repo, entry, branch) = fixture();
            match missing {
                "main_branch" => {
                    engine.state.branches.remove(&repo.main_branch_id);
                }
                "main_head" => {
                    let head = engine.state.branches[&repo.main_branch_id]
                        .head_commit_id
                        .clone();
                    engine.state.commits.remove(&head);
                }
                "file_head" => {
                    engine
                        .state
                        .branches
                        .get_mut(&branch)
                        .unwrap()
                        .head_commit_id = "missing".into();
                }
                "history" => {
                    engine.state.histories.remove(&repo.history_tree_id);
                }
                _ => unreachable!(),
            }
            let before = canonical_json(&engine.state).unwrap();
            let result = if scoped {
                engine.select_file_branch_at(
                    &repo.repository_id,
                    &entry.path,
                    &entry.file_id,
                    &branch,
                    "admin",
                    &new_id("op"),
                )
            } else {
                engine.select_file_branch(&entry.file_id, &branch, "admin", &new_id("op"))
            };
            let error = result.unwrap_err();
            assert!(
                matches!(
                    error.code(),
                    "STATE_INVALID" | "BRANCH_NOT_FOUND" | "HISTORY_NOT_FOUND"
                ),
                "{missing}/{scoped}: {error}"
            );
            assert_eq!(
                canonical_json(&engine.state).unwrap(),
                before,
                "{missing}/{scoped}"
            );
        }
    }
}

#[test]
fn workspace_missing_head_returns_error_without_creating_workspace() {
    let (_temporary, mut engine, repo, _, _) = fixture();
    let head = engine.state.branches[&repo.main_branch_id]
        .head_commit_id
        .clone();
    engine.state.commits.remove(&head);
    let shared = std::sync::Mutex::new(engine);
    let mut engine = shared.lock().unwrap();
    let before = canonical_json(&engine.state).unwrap();
    let workspaces = engine.workspaces.len();
    let error = engine
        .create_workspace(&repo.main_branch_id, "admin")
        .unwrap_err();
    assert_eq!(error.code(), "STATE_INVALID");
    assert_eq!(engine.workspaces.len(), workspaces);
    assert_eq!(canonical_json(&engine.state).unwrap(), before);
    drop(engine);
    assert!(
        shared.lock().is_ok(),
        "recoverable corruption must not poison the shared engine"
    );
}

#[test]
fn batch_rejects_missing_references_without_publication() {
    for missing in ["main_branch", "main_head", "scope_commit", "owner"] {
        let (_temporary, mut engine, repo, entry, _) = fixture();
        let result = engine
            .create_directory_branches(
                &repo.repository_id,
                "folder",
                "draft",
                "admin",
                &new_id("op"),
            )
            .unwrap();
        let branch_id = result.result["directory_branch_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let branch = engine.state.directory_branches[&branch_id].clone();
        let request = DirectoryNamespaceBatch {
            kind: "rename".into(),
            items: vec![DirectoryNamespaceItem {
                source: DirectoryNamespaceSource {
                    directory_branch_id: branch_id.clone(),
                    expected_tree_id: branch.head_tree_id.clone(),
                    path: entry.path,
                    entry_id: entry.entry_id,
                },
                target: Some(DirectoryNamespaceTarget {
                    directory_branch_id: branch_id.clone(),
                    expected_tree_id: branch.head_tree_id,
                    path: "folder/renamed.txt".into(),
                }),
            }],
        };
        match missing {
            "main_branch" => {
                engine.state.branches.remove(&repo.main_branch_id);
            }
            "main_head" => {
                let head = engine.state.branches[&repo.main_branch_id]
                    .head_commit_id
                    .clone();
                engine.state.commits.remove(&head);
            }
            "scope_commit" => {
                engine
                    .state
                    .directory_branches
                    .get_mut(&branch_id)
                    .unwrap()
                    .commit_id = "missing".into();
            }
            "owner" => {
                let branch = engine.state.directory_branches.get_mut(&branch_id).unwrap();
                branch.file_contexts.clear();
                branch.file_branches.clear();
            }
            _ => unreachable!(),
        }
        let before = canonical_json(&engine.state).unwrap();
        let error = engine
            .directory_namespace_batch(&repo.repository_id, &request, "admin", &new_id("op"))
            .unwrap_err();
        assert_eq!(error.code(), "STATE_INVALID", "{missing}");
        assert_eq!(canonical_json(&engine.state).unwrap(), before, "{missing}");
    }
}

#[test]
fn file_node_publication_rejects_missing_main_without_leaking_new_nodes() {
    for missing in ["main_branch", "main_head"] {
        let (_temporary, mut engine, repo, entry, branch) = fixture();
        match missing {
            "main_branch" => {
                engine.state.branches.remove(&repo.main_branch_id);
            }
            "main_head" => {
                let head = engine.state.branches[&repo.main_branch_id]
                    .head_commit_id
                    .clone();
                engine.state.commits.remove(&head);
            }
            _ => unreachable!(),
        }
        let before = canonical_json(&engine.state).unwrap();
        let error = engine
            .update_file_node_tags(
                &entry.file_id,
                &[FileTagTarget {
                    branch_id: branch,
                    source_node_id: entry.version_id.clone(),
                    expected_node_id: entry.version_id,
                }],
                &["tag".into()],
                &[],
                "admin",
                &new_id("op"),
            )
            .unwrap_err();
        assert_eq!(error.code(), "STATE_INVALID", "{missing}");
        assert_eq!(canonical_json(&engine.state).unwrap(), before, "{missing}");
    }
}

#[test]
fn missing_file_nodes_and_bodies_return_errors_without_publication() {
    for missing in ["node", "body"] {
        let (_temporary, mut engine, _, entry, branch) = fixture();
        let head = if missing == "node" {
            engine.state.branches.get_mut(&branch).unwrap().file_node_id = Some("missing".into());
            "missing".to_owned()
        } else {
            let node = new_id("fnode");
            engine.state.file_path_nodes.insert(
                node.clone(),
                FilePathNode {
                    node_id: node.clone(),
                    history_tree_id: engine.state.branches[&branch].history_tree_id.clone(),
                    file_id: entry.file_id.clone(),
                    version_id: "missing".into(),
                    parent_node_ids: vec![entry.version_id],
                    branch_id: branch.clone(),
                    path: entry.path.clone(),
                    previous_path: entry.path,
                    sequence: engine.state.sequence,
                    created_at: now(),
                    operation_id: new_id("op"),
                    author: "admin".into(),
                },
            );
            engine.state.branches.get_mut(&branch).unwrap().file_node_id = Some(node.clone());
            node
        };
        let before = canonical_json(&engine.state).unwrap();
        let error = engine
            .update_file_node_tags(
                &entry.file_id,
                &[FileTagTarget {
                    branch_id: branch,
                    source_node_id: head.clone(),
                    expected_node_id: head.clone(),
                }],
                &["tag".into()],
                &[],
                "admin",
                &new_id("op"),
            )
            .unwrap_err();
        assert!(
            matches!(error.code(), "STATE_INVALID" | "FILE_NODE_NOT_FOUND"),
            "{missing}: {error}"
        );
        assert_eq!(canonical_json(&engine.state).unwrap(), before, "{missing}");
        let error = engine
            .create_file_branch_from_node(
                &entry.file_id,
                "missing",
                &head,
                "broken source",
                "admin",
                &new_id("op"),
            )
            .unwrap_err();
        assert_eq!(
            error.code(),
            if missing == "body" {
                "STATE_INVALID"
            } else {
                "FILE_NODE_NOT_FOUND"
            }
        );
        assert_eq!(
            canonical_json(&engine.state).unwrap(),
            before,
            "source/{missing}"
        );
    }
}

#[test]
fn exhausted_branch_generations_reject_metadata_mutations_atomically() {
    for action in [
        "select",
        "select_at",
        "namespace",
        "tags_file",
        "tags_main",
        "archive",
        "merge_survivor",
        "merge_alias",
    ] {
        let (_temporary, mut engine, repo, entry, draft) = fixture();
        let owner = engine.state.file_selections[&entry.file_id].clone();
        let saturated = match action {
            "tags_file" | "archive" | "merge_alias" => &draft,
            "merge_survivor" => &owner,
            _ => &repo.main_branch_id,
        };
        engine.state.branches.get_mut(saturated).unwrap().generation = u64::MAX;
        let before = canonical_json(&engine.state).unwrap();
        let operation = new_id("op");
        let result = match action {
            "select" => engine.select_file_branch(&entry.file_id, &draft, "admin", &operation),
            "select_at" => engine.select_file_branch_at(
                &repo.repository_id,
                &entry.path,
                &entry.file_id,
                &draft,
                "admin",
                &operation,
            ),
            "namespace" => engine.file_namespace(
                &repo.repository_id,
                NamespaceOperation {
                    kind: "rename".into(),
                    path: entry.path.clone(),
                    destination: Some("folder/renamed.txt".into()),
                },
                "admin",
                &operation,
            ),
            "tags_file" | "tags_main" => engine.update_file_node_tags(
                &entry.file_id,
                &[FileTagTarget {
                    branch_id: if action == "tags_file" {
                        draft.clone()
                    } else {
                        owner.clone()
                    },
                    source_node_id: entry.version_id.clone(),
                    expected_node_id: entry.version_id.clone(),
                }],
                &["tag".into()],
                &[],
                "admin",
                &operation,
            ),
            "archive" => engine.archive_branch(&draft, "admin", &operation),
            "merge_survivor" | "merge_alias" => engine.merge_file_branches(
                &entry.file_id,
                &[
                    FileMergeInput {
                        branch_id: owner.clone(),
                        expected_node_id: entry.version_id.clone(),
                    },
                    FileMergeInput {
                        branch_id: draft.clone(),
                        expected_node_id: entry.version_id.clone(),
                    },
                ],
                &entry.version_id,
                &owner,
                "admin",
                &operation,
            ),
            _ => unreachable!(),
        };
        assert_eq!(result.unwrap_err().code(), "RESOURCE_LIMIT", "{action}");
        assert_eq!(canonical_json(&engine.state).unwrap(), before, "{action}");
    }
}

#[test]
fn exhausted_workspace_generations_reject_candidate_without_publication() {
    for saturated_owner in [false, true] {
        let (_temporary, mut engine, repo, entry, _) = fixture();
        let owner = engine.state.file_selections[&entry.file_id].clone();
        engine
            .state
            .branches
            .get_mut(if saturated_owner {
                &owner
            } else {
                &repo.main_branch_id
            })
            .unwrap()
            .generation = u64::MAX;
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, &entry.path, b"changed body")
            .unwrap();
        let before = canonical_json(&engine.state).unwrap();
        let error = engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "must not publish".into(),
                request_digest: None,
            })
            .unwrap_err();
        assert_eq!(error.code(), "RESOURCE_LIMIT", "owner={saturated_owner}");
        assert_eq!(
            canonical_json(&engine.state).unwrap(),
            before,
            "owner={saturated_owner}"
        );
    }
}

#[test]
fn exhausted_main_generation_rejects_active_namespace_batch() {
    let (_temporary, mut engine, repo, entry, _) = fixture();
    let directory = engine
        .state
        .directory_branches
        .values()
        .find(|branch| {
            branch.repository_id == repo.repository_id
                && branch.path == "folder"
                && engine.state.directory_selections.get(&branch.directory_id)
                    == Some(&branch.directory_branch_id)
        })
        .unwrap()
        .clone();
    let request = DirectoryNamespaceBatch {
        kind: "rename".into(),
        items: vec![DirectoryNamespaceItem {
            source: DirectoryNamespaceSource {
                directory_branch_id: directory.directory_branch_id.clone(),
                expected_tree_id: directory.head_tree_id.clone(),
                path: entry.path,
                entry_id: entry.entry_id,
            },
            target: Some(DirectoryNamespaceTarget {
                directory_branch_id: directory.directory_branch_id,
                expected_tree_id: directory.head_tree_id,
                path: "folder/renamed.txt".into(),
            }),
        }],
    };
    engine
        .state
        .branches
        .get_mut(&repo.main_branch_id)
        .unwrap()
        .generation = u64::MAX;
    let before = canonical_json(&engine.state).unwrap();
    let error = engine
        .directory_namespace_batch(&repo.repository_id, &request, "admin", &new_id("op"))
        .unwrap_err();
    assert_eq!(error.code(), "RESOURCE_LIMIT");
    assert_eq!(canonical_json(&engine.state).unwrap(), before);
}
