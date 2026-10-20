//! Pure scope validation and transaction payload construction.
use super::*;
pub(super) fn valid_path(path: &str) -> bool {
    path.is_empty() || path.split('/').all(valid_name)
}
fn error_missing() -> String {
    t(
        "所选分支中有条目已不存在或依据子树缺失，请取消并刷新后重试。",
        "An entry or its base subtree is missing in a selected branch. Cancel and refresh before retrying.",
    )
}
fn branch_ref(branch: &Value, entry: &Value) -> Result<Value, String> {
    let id = branch_id(branch);
    let tree = text(branch, &["head_tree_id"]);
    let entry_id = entry_id(entry);
    let path = text(entry, &["path"]);
    if id.is_empty() || tree.is_empty() || entry_id.is_empty() || path.is_empty() {
        return Err(error_missing());
    }
    Ok(json!({"directory_branch_id":id,"expected_tree_id":tree,"path":path,"entry_id":entry_id}))
}
fn find_entry<'a>(branch: &'a Value, chosen: &Value) -> Result<&'a Value, String> {
    let id = entry_id(chosen);
    let matches = branch["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry["entry_id"] == id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if matches.len() == 1 {
        return Ok(matches[0]);
    }
    let logical_path = chosen["repository_path"]
        .as_str()
        .or_else(|| chosen["path"].as_str())
        .unwrap_or_default();
    matches
        .into_iter()
        .find(|entry| entry["path"] == logical_path)
        .ok_or_else(error_missing)
}
fn target_ref(branch: &Value, path: String) -> Result<Value, String> {
    let id = branch_id(branch);
    let tree = text(branch, &["head_tree_id"]);
    if id.is_empty() || tree.is_empty() {
        return Err(error_missing());
    }
    Ok(json!({"directory_branch_id":id,"expected_tree_id":tree,"path":path}))
}
/// Scope and identity are captured before submission. Never infer all possible
/// source/target pairs or silently skip an entry absent in another branch.
pub(super) fn request(draft: &Draft) -> Result<Value, String> {
    if draft.selected_sources.is_empty() {
        return Err(t(
            "至少选择一个源分支。",
            "Select at least one source branch.",
        ));
    }
    if draft.entries.is_empty() {
        return Err(error_missing());
    }
    let mut items = vec![];
    let mut unique = BTreeSet::new();
    if draft.kind.has_destination() {
        if draft.mappings.is_empty()
            || draft.selected_sources.iter().any(|source| {
                !draft
                    .mappings
                    .iter()
                    .any(|mapping| &mapping.source == source)
            })
        {
            return Err(t(
                "请为每个源分支指定目标映射。",
                "Specify a destination mapping for every source branch.",
            ));
        }
        for mapping in &draft.mappings {
            if !draft.selected_sources.contains(&mapping.source) {
                return Err(error_missing());
            }
            let source = draft
                .sources
                .iter()
                .find(|branch| branch_id(branch) == mapping.source)
                .ok_or_else(error_missing)?;
            let target = draft
                .targets
                .iter()
                .find(|branch| branch_id(branch) == mapping.target)
                .ok_or_else(|| {
                    t(
                        "请明确选择每条映射的目标分支。",
                        "Explicitly select the destination branch for each mapping.",
                    )
                })?;
            for chosen in &draft.entries {
                let id = entry_id(chosen);
                let entry = find_entry(source, chosen)?;
                let name = draft
                    .names
                    .get(&entry_key(chosen))
                    .ok_or_else(error_missing)?;
                validate_name(name)?;
                let destination = join_path(&text(target, &["path"]), name);
                let key = (
                    mapping.source.clone(),
                    id,
                    mapping.target.clone(),
                    destination.clone(),
                );
                if !unique.insert(key) {
                    return Err(t(
                        "存在重复映射，请先移除。",
                        "Remove the duplicate mapping.",
                    ));
                }
                items.push(json!({"source":branch_ref(source,entry)?,"target":target_ref(target,destination)?}));
            }
        }
    } else {
        for source_id in &draft.selected_sources {
            let source = draft
                .sources
                .iter()
                .find(|branch| branch_id(branch) == *source_id)
                .ok_or_else(error_missing)?;
            for chosen in &draft.entries {
                let entry = find_entry(source, chosen)?;
                let mut item = json!({"source":branch_ref(source,entry)?});
                if draft.kind == Kind::Rename {
                    let name = draft
                        .names
                        .get(&entry_key(chosen))
                        .ok_or_else(error_missing)?;
                    validate_name(name)?;
                    let path = text(entry, &["path"]);
                    let parent = path
                        .rsplit_once('/')
                        .map(|(parent, _)| parent)
                        .unwrap_or_default();
                    item["target"] = target_ref(source, join_path(parent, name))?;
                }
                items.push(item);
            }
        }
    }
    Ok(json!({"kind":draft.kind.wire(),"items":items}))
}
fn validate_name(name: &str) -> Result<(), String> {
    if name.len() <= zxs_contract::MAX_NOTE_BYTES && valid_name(name) {
        Ok(())
    } else {
        Err(t(
            "名称无效，最多 128 个 UTF-8 字节。",
            "Invalid name. Use at most 128 UTF-8 bytes.",
        ))
    }
}
#[cfg(test)]
mod tests {
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
}
