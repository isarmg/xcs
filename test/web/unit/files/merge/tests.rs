use super::*;
fn draft() -> Draft {
    Draft {
        file: "file".into(),
        repository: "repo".into(),
        path: "a.txt".into(),
        branches: vec![
            json!({"branch_id":"left","node_id":"fnode-left"}),
            json!({"branch_id":"right","node_id":"fnode-right"}),
            json!({"branch_id":"alias","node_id":"fnode-old","read_only":true}),
        ],
        nodes: vec![
            json!({"node_id":"fnode-unrelated","version_id":"body-old"}),
            json!({"node_id":"fnode-left","version_id":"body-left"}),
        ],
        selected: BTreeSet::from(["left".into(), "right".into()]),
        chosen: "fnode-unrelated".into(),
        survivor: String::new(),
    }
}
#[test]
fn merging_requires_an_explicit_survivor_and_accepts_any_file_node() {
    let mut draft = draft();
    assert!(request(&draft).is_err());
    draft.survivor = "left".into();
    let body = request(&draft).unwrap();
    assert_eq!(body["chosen_node_id"], "fnode-unrelated");
    assert_eq!(body["inputs"][0]["expected_node_id"], "fnode-left");
    assert_eq!(body["surviving_branch_id"], "left");
    assert_eq!(body["confirmed"], true);
    assert!(body.get("content").is_none());
}
#[test]
fn aliases_and_missing_baselines_cannot_be_submitted() {
    let mut draft = draft();
    draft.survivor = "left".into();
    draft.selected.insert("alias".into());
    assert!(request(&draft).is_err());
    draft.selected.remove("alias");
    draft.branches[0]["node_id"] = Value::Null;
    assert!(request(&draft).is_err());
}
