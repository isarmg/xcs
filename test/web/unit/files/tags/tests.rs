use super::*;
#[test]
fn tag_deltas_preserve_heterogeneous_inherited_labels_and_do_not_mutate_the_parent() {
    let node = json!({"tags":["old","keep"]});
    let updated = result_tags(
        &node,
        &BTreeSet::from(["new".into()]),
        &BTreeSet::from(["old".into()]),
    );
    assert_eq!(updated, BTreeSet::from(["keep".into(), "new".into()]));
    assert_eq!(node["tags"], json!(["old", "keep"]));
    assert!(valid_tag(&format!("{}ab", "文".repeat(42))));
    assert!(!valid_tag(&"文".repeat(43)));
    assert!(!valid_tag("bad\nlabel"));
}
#[test]
fn requests_distinguish_historical_source_nodes_from_frozen_branch_heads() {
    let draft = Draft {
        file: "file".into(),
        repository: "repo".into(),
        path: "a.txt".into(),
        nodes: vec![json!({"node_id":"old-node","version_id":"old-body","tags":["inherited"]})],
        branches: vec![json!({"branch_id":"main","node_id":"newer-head"})],
        targets: vec![Target {
            source: "old-node".into(),
            branch: "main".into(),
            expected: "newer-head".into(),
        }],
        add: BTreeSet::from(["reviewed".into()]),
        remove: BTreeSet::new(),
        input: String::new(),
    };
    let body = request(&draft).unwrap();
    assert_eq!(body["targets"][0]["source_node_id"], "old-node");
    assert_eq!(body["targets"][0]["expected_node_id"], "newer-head");
    assert_eq!(body["add"], json!(["reviewed"]));
    assert!(body.get("note").is_none());
}

#[test]
fn one_invalid_tag_target_blocks_the_entire_batch() {
    let labels = (0..storage_contract::MAX_FILE_TAGS)
        .map(|index| format!("tag-{index}"))
        .collect::<Vec<_>>();
    let mut draft = Draft {
        file: "file".into(),
        repository: "repo".into(),
        path: "a.txt".into(),
        nodes: vec![
            json!({"node_id":"old","tags":[]}),
            json!({"node_id":"full","tags":labels}),
        ],
        branches: vec![json!({"branch_id":"main","node_id":"head"})],
        targets: vec![
            Target {
                source: "old".into(),
                branch: "main".into(),
                expected: "head".into(),
            },
            Target {
                source: "full".into(),
                branch: "main".into(),
                expected: "head".into(),
            },
        ],
        add: BTreeSet::from(["extra".into()]),
        remove: BTreeSet::new(),
        input: String::new(),
    };
    assert!(
        request(&draft).is_err(),
        "a valid first target must not hide a full second target"
    );
    draft.remove.insert("tag-0".into());
    assert_eq!(
        request(&draft).unwrap()["targets"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    draft.branches[0]["alias_of"] = "survivor".into();
    assert!(
        request(&draft).is_err(),
        "read-only aliases cannot target edits even without a redundant read_only flag"
    );
}
