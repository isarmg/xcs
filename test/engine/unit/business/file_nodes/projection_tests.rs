use super::*;

fn path(state: &mut State, id: &str, parents: &[&str]) {
    state.file_path_nodes.insert(
        id.into(),
        FilePathNode {
            node_id: id.into(),
            history_tree_id: "history".into(),
            file_id: "file".into(),
            version_id: "body".into(),
            parent_node_ids: parents.iter().map(|p| (*p).into()).collect(),
            branch_id: "branch".into(),
            path: "new".into(),
            previous_path: "old".into(),
            operation_id: "operation".into(),
            author: "author".into(),
            sequence: 1,
            created_at: 1,
        },
    );
}

fn metadata(state: &mut State, id: &str, parents: &[&str], tags: &[&str]) {
    state.file_metadata_nodes.insert(
        id.into(),
        FileMetadataNode {
            node_id: id.into(),
            history_tree_id: "history".into(),
            file_id: "file".into(),
            version_id: "body".into(),
            parent_node_ids: parents.iter().map(|p| (*p).into()).collect(),
            branch_id: "branch".into(),
            kind: "tags".into(),
            tags: tags.iter().map(|t| (*t).into()).collect(),
            operation_id: "operation".into(),
            author: "author".into(),
            sequence: 1,
            created_at: 1,
        },
    );
}

fn merge(state: &mut State, id: &str, result: &str, parents: &[&str]) {
    state.file_branch_merges.insert(
        id.into(),
        FileBranchMerge {
            merge_id: id.into(),
            history_tree_id: "history".into(),
            file_id: "file".into(),
            input_branch_ids: Vec::new(),
            input_node_ids: Vec::new(),
            chosen_node_id: result.into(),
            result_node_id: result.into(),
            surviving_branch_id: "branch".into(),
            reused_node: true,
            parent_node_ids: parents.iter().map(|p| (*p).into()).collect(),
            operation_id: "operation".into(),
            sequence: 1,
        },
    );
}

#[test]
fn projection_matches_uncached_nodes_and_serialized_bytes() {
    let mut state = State::default();
    metadata(&mut state, "tagged", &["tagged"], &["z", "a"]);
    metadata(&mut state, "empty", &["tagged"], &[]);
    path(&mut state, "tagged", &["missing"]); // metadata wins over a path ID
    path(&mut state, "left", &["tagged"]);
    path(&mut state, "right", &["empty"]);
    path(&mut state, "merged", &["left", "left", "right"]);
    path(&mut state, "cycle-a", &["cycle-b"]);
    path(&mut state, "cycle-b", &["cycle-a"]);
    path(&mut state, "cycle-tail", &["cycle-a", "tagged"]);
    path(&mut state, "dangling", &["missing"]);
    // Reverse insertion order must not change BTreeMap merge order.
    merge(&mut state, "z", "merged", &["other", "left", "merged"]);
    merge(&mut state, "a", "merged", &["right", "first", "first"]);
    merge(&mut state, "unrelated", "elsewhere", &["tagged"]);
    for (id, parents) in [
        ("body", vec!["left"]),
        ("source-only", vec!["left"]),
        ("override", vec!["right"]),
        ("empty-override", vec!["left"]),
    ] {
        state.versions.insert(
            id.into(),
            FileVersion {
                history_tree_id: "history".into(),
                version_id: id.into(),
                file_id: "file".into(),
                creation_primary_slot_id: "P01".into(),
                logical_size: 0,
                version_sequence: 1,
                content_hash: "0".repeat(64),
                checksum_chunk_bytes: 262144,
                checksum_manifest_hash: "0".repeat(64),
                creation_name: "body".into(),
                storage_name: "v000001_20261020T000000Z_00000000.txt".into(),
                created_at: "2026-10-20T00:00:00Z".into(),
                source_version_ids: parents.into_iter().map(str::to_owned).collect(),
                created_as_full_checkpoint: true,
            },
        );
    }
    state
        .file_node_parents
        .insert("override".into(), vec!["left".into()]);
    state
        .file_node_parents
        .insert("empty-override".into(), Vec::new());
    // A path node takes precedence over a body with the same ID.
    path(&mut state, "body", &["right"]);
    let nodes = [
        "body",
        "source-only",
        "override",
        "empty-override",
        "merged",
        "right",
        "left",
        "tagged",
        "empty",
        "cycle-tail",
        "cycle-b",
        "cycle-a",
        "dangling",
        "missing",
    ];
    let mut projection = NodeProjection::new(&state);
    for node in nodes.into_iter().chain(nodes.into_iter().rev()) {
        let expected = serde_json::json!({"parent_node_ids": node_parents(&state, node), "tags": node_tags(&state, node)});
        let actual = serde_json::json!({"parent_node_ids": projection.parents(node), "tags": projection.tags(node)});
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&expected).unwrap(),
            "{node}"
        );
    }
    assert_eq!(
        projection.parents("merged"),
        ["left", "left", "right", "first", "other"]
    );
}

#[test]
fn projection_walks_a_linear_history_once_in_either_query_order() {
    const LENGTH: usize = 4096;
    let mut state = State::default();
    metadata(&mut state, "0", &[], &["tag"]);
    for n in 1..LENGTH {
        path(&mut state, &n.to_string(), &[&(n - 1).to_string()]);
    }
    for reverse in [false, true] {
        let mut projection = NodeProjection::new(&state);
        for n in 0..LENGTH {
            let id = if reverse { LENGTH - n - 1 } else { n }.to_string();
            assert_eq!(projection.tags(&id), &BTreeSet::from(["tag".into()]));
        }
        assert_eq!(projection.tag_visits, LENGTH);
        for n in 0..LENGTH {
            projection.tags(&n.to_string());
        }
        assert_eq!(
            projection.tag_visits, LENGTH,
            "replica lookups must not traverse again"
        );
    }
}
