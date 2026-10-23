use super::*;
fn file(id: &str, path: &str, version: &str) -> Value {
    json!({"entry_id":id,"file_id":id,"path":path,"name":path.rsplit('/').next().unwrap(),"kind":"file","version_id":version})
}
fn branch(id: &str, path: &str, entries: Vec<Value>) -> Value {
    json!({"directory_branch_id":id,"head_tree_id":format!("tree-{id}"),"name":id,"path":path,"entries":entries})
}
fn draft(kind: Kind) -> Draft {
    let f = file("f", "a/old.txt", "F1");
    let g = file("g", "a/data.txt", "G1");
    Draft {
        id: "draft".into(),
        kind,
        repository: "repo".into(),
        repository_path: "a".into(),
        disk_prefix: String::new(),
        entries: vec![f.clone(), g.clone()],
        names: BTreeMap::from([
            (entry_key(&file("f", "a/old.txt", "F1")), "new.txt".into()),
            (entry_key(&file("g", "a/data.txt", "G1")), "data.txt".into()),
        ]),
        sources: vec![
            branch("m", "a", vec![f, g]),
            branch(
                "n",
                "a",
                vec![
                    file("f", "a/alias.txt", "F2"),
                    file("g", "a/data.txt", "G2"),
                ],
            ),
            branch("r", "a", vec![]),
        ],
        selected_sources: BTreeSet::from(["m".into(), "n".into()]),
        catalogue: vec![],
        target_path: "b".into(),
        target_resolved: true,
        targets: vec![branch("l", "b", vec![]), branch("p", "b", vec![])],
        mappings: vec![
            Mapping {
                source: "m".into(),
                target: "l".into(),
            },
            Mapping {
                source: "n".into(),
                target: "p".into(),
            },
        ],
        loading: false,
        submitted_operation: None,
        generation: 0,
        error: String::new(),
    }
}
#[test]
fn uncertain_results_keep_the_original_operation_until_its_own_receipt() {
    let mut draft = draft(Kind::Copy);
    draft.submitted_operation = Some("operation-one".into());
    let mut model = Model {
        batch: Some(draft),
        checked_entries: BTreeSet::from(["f".into()]),
        ..Default::default()
    };
    settle_operation(
        &mut model,
        "operation-one",
        crate::files::model::SaveStatus::Unknown,
    );
    settle_operation(
        &mut model,
        "unrelated",
        crate::files::model::SaveStatus::Published,
    );
    assert!(model.batch.is_some());
    settle_operation(
        &mut model,
        "operation-one",
        crate::files::model::SaveStatus::Rejected,
    );
    assert!(model.batch.as_ref().unwrap().submitted_operation.is_none());
    model.batch.as_mut().unwrap().submitted_operation = Some("operation-two".into());
    settle_operation(
        &mut model,
        "operation-one",
        crate::files::model::SaveStatus::Published,
    );
    assert!(model.batch.is_some());
    settle_operation(
        &mut model,
        "operation-two",
        crate::files::model::SaveStatus::Published,
    );
    assert!(model.batch.is_none());
    assert!(model.checked_entries.is_empty());
}
#[test]
fn multiple_aliases_of_one_identity_resolve_by_path_without_sharing_name_edits() {
    let mut draft = draft(Kind::Rename);
    draft.selected_sources = BTreeSet::from(["m".into()]);
    let first = file("f", "a/first.txt", "F1");
    let second = file("f", "a/second.txt", "F2");
    draft.entries = vec![first.clone(), second.clone()];
    draft.sources[0]["entries"] = json!([first, second]);
    draft.names = BTreeMap::from([
        (entry_key(&draft.entries[0]), "renamed-first.txt".into()),
        (entry_key(&draft.entries[1]), "renamed-second.txt".into()),
    ]);
    let body = request(&draft).unwrap();
    assert_eq!(body["items"][0]["source"]["path"], "a/first.txt");
    assert_eq!(body["items"][1]["source"]["path"], "a/second.txt");
    assert_eq!(body["items"][0]["target"]["path"], "a/renamed-first.txt");
    assert_eq!(body["items"][1]["target"]["path"], "a/renamed-second.txt");
}
#[test]
fn paired_moves_keep_per_branch_paths_and_frozen_tree_guards() {
    let body = request(&draft(Kind::Move)).unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(
        items[0]["source"],
        json!({"directory_branch_id":"m","expected_tree_id":"tree-m","entry_id":"f","path":"a/old.txt"})
    );
    assert_eq!(
        items[0]["target"],
        json!({"directory_branch_id":"l","expected_tree_id":"tree-l","path":"b/new.txt"})
    );
    assert_eq!(items[2]["source"]["path"], "a/alias.txt");
    assert_eq!(items[2]["target"]["directory_branch_id"], "p");
    assert!(
        items
            .iter()
            .all(|item| item["source"]["directory_branch_id"] != "r")
    );
}
#[test]
fn copying_to_multiple_explicit_targets_never_infers_a_cartesian_product() {
    let mut draft = draft(Kind::Copy);
    draft.selected_sources = BTreeSet::from(["m".into()]);
    draft.entries.truncate(1);
    draft.mappings = vec![
        Mapping {
            source: "m".into(),
            target: "l".into(),
        },
        Mapping {
            source: "m".into(),
            target: "p".into(),
        },
    ];
    assert_eq!(
        request(&draft).unwrap()["items"].as_array().unwrap().len(),
        2
    );
    draft.mappings.push(draft.mappings[0].clone());
    assert!(request(&draft).unwrap_err().contains("duplicate"));
}
#[test]
fn an_absent_entry_rejects_the_entire_selected_scope() {
    let mut draft = draft(Kind::Delete);
    draft.sources[1]["entries"] = json!([file("f", "a/alias.txt", "F2")]);
    assert!(request(&draft).is_err());
    draft.sources[1]["entries"] = json!([
        file("f", "a/alias.txt", "F2"),
        file("g", "a/data.txt", "G2")
    ]);
    assert_eq!(
        request(&draft).unwrap()["items"].as_array().unwrap().len(),
        4
    );
}
#[test]
fn rename_and_delete_stay_inside_each_chosen_branch() {
    let mut draft = draft(Kind::Rename);
    let body = request(&draft).unwrap();
    assert_eq!(body["items"][2]["source"]["path"], "a/alias.txt");
    assert_eq!(body["items"][2]["target"]["path"], "a/new.txt");
    assert_eq!(body["items"][2]["target"]["directory_branch_id"], "n");
    draft.kind = Kind::Delete;
    assert!(
        request(&draft).unwrap()["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.get("target").is_none())
    );
}
#[test]
fn no_mapping_missing_guard_or_invalid_utf8_byte_length_is_submitted() {
    let mut draft = draft(Kind::Move);
    draft.mappings[1].target.clear();
    assert!(request(&draft).is_err());
    draft.mappings[1].target = "p".into();
    draft.sources[0]["head_tree_id"] = Value::Null;
    assert!(request(&draft).is_err());
    draft.sources[0]["head_tree_id"] = "tree-m".into();
    draft
        .names
        .insert(entry_key(&draft.entries[0]), "文".repeat(43));
    assert!(request(&draft).is_err());
    draft.names.insert(
        entry_key(&draft.entries[0]),
        format!("{}ab", "文".repeat(42)),
    );
    assert!(request(&draft).is_ok());
    for bad in ["../b", "a//b", "a\\b", "a/./b"] {
        assert!(!valid_path(bad));
    }
    assert!(valid_path("资料/报告"));
}

#[test]
fn stale_detail_rename_keeps_original_identity_and_tree_instead_of_retargeting_same_path() {
    let mut draft = draft(Kind::Rename);
    draft.entries.truncate(1);
    draft.selected_sources = BTreeSet::from(["m".into()]);
    let body = request(&draft).unwrap();
    assert_eq!(
        body["items"][0]["source"],
        json!({"directory_branch_id":"m","expected_tree_id":"tree-m","entry_id":"f","path":"a/old.txt"})
    );
    assert_eq!(
        body["items"][0]["target"],
        json!({"directory_branch_id":"m","expected_tree_id":"tree-m","path":"a/new.txt"})
    );
    // If scope loading sees a replacement at the old path, fail closed rather
    // than constructing a path-only rename for the replacement identity.
    draft.sources[0]["entries"] = json!([file("replacement", "a/old.txt", "replacement-version")]);
    draft.sources[0]["head_tree_id"] = "replacement-tree".into();
    assert!(request(&draft).is_err());
}

#[test]
fn target_reads_preserve_the_draft_when_the_session_is_inactive() {
    Owner::new().with(|| {
        for (authenticated, session_transition) in [
            (false, crate::ui::SessionTransition::Idle),
            (true, crate::ui::SessionTransition::Restoring),
            (true, crate::ui::SessionTransition::LoggingOut),
            (true, crate::ui::SessionTransition::LogoutUnconfirmed),
        ] {
            let state = RwSignal::new(State {
                authenticated,
                session_transition,
                ..Default::default()
            });
            state.update(|s| s.files.batch = Some(draft(Kind::Copy)));
            super::super::actions::load_target(state);
            state.with_untracked(|s| {
                let draft = s.files.batch.as_ref().unwrap();
                assert_eq!(draft.generation, 0);
                assert!(!draft.loading);
                assert!(draft.target_resolved);
                assert_eq!(draft.targets.len(), 2);
            });
        }
    });
}
