//! Explicit multi-branch merge review. Inputs and their heads are frozen when
//! opening the panel; the chosen result may be any node of this file.
use super::{
    actions,
    model::{Model, history_node_id, history_nodes, selected_branch},
    review,
};
use crate::{
    api, format,
    ui::{State, text},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use zxs_web::{i18n::t, prelude::*};
mod view;
pub(crate) use view::{history, view};

#[derive(Clone)]
pub(crate) struct Draft {
    file: String,
    repository: String,
    path: String,
    branches: Vec<Value>,
    nodes: Vec<Value>,
    selected: BTreeSet<String>,
    chosen: String,
    survivor: String,
}
pub(crate) use super::model::branch_writable;
fn branch_id(branch: &Value) -> String {
    text(branch, &["branch_id"])
}
pub(super) fn branch_head(branch: &Value) -> String {
    branch["node_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .or_else(|| branch["head_node_id"].as_str())
        .or_else(|| branch["version_id"].as_str())
        .unwrap_or_default()
        .to_owned()
}
pub(crate) fn available(model: &Model) -> bool {
    !model.history_loading
        && model.file_history["branches"]
            .as_array()
            .is_some_and(|branches| {
                branches
                    .iter()
                    .filter(|branch| branch_writable(branch))
                    .count()
                    >= 2
            })
}
pub(crate) fn start(state: RwSignal<State>) {
    let snapshot = state.get_untracked();
    if !snapshot.files.writable
        || super::model::selected_entry_read_only(&snapshot.files)
        || actions::pending(&snapshot)
        || !available(&snapshot.files)
    {
        return;
    }
    let Some(entry) = snapshot.files.selected.as_ref() else {
        return;
    };
    let file = text(entry, &["file_id"]);
    let Some((repository, path)) = super::model::file_context(&snapshot.files, entry) else {
        return;
    };
    let branches = snapshot.files.file_history["branches"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(branch_writable)
        .collect::<Vec<_>>();
    let current = selected_branch(&snapshot.files);
    let selected = branches
        .iter()
        .filter(|branch| branch_id(branch) == current)
        .map(branch_id)
        .collect();
    let draft = Draft {
        file,
        repository,
        path,
        branches,
        nodes: history_nodes(&snapshot.files.file_history),
        selected,
        chosen: String::new(),
        survivor: String::new(),
    };
    review::reset(state, review::Kind::Merge);
    state.update(|s| s.files.merge = Some(draft));
}
fn request(draft: &Draft) -> Result<Value, String> {
    if draft.selected.len() < 2 {
        return Err(t(
            "至少选择两个可写文件分支。",
            "Select at least two writable file branches.",
        ));
    }
    let mut inputs = vec![];
    for id in &draft.selected {
        let branch = draft
            .branches
            .iter()
            .find(|branch| branch_id(branch) == *id && branch_writable(branch))
            .ok_or_else(|| {
                t(
                    "分支不可用，请取消后刷新。",
                    "A branch is unavailable. Cancel and refresh.",
                )
            })?;
        let head = branch_head(branch);
        if head.is_empty() {
            return Err(t(
                "分支依据节点缺失，请取消后刷新。",
                "A branch base node is missing. Cancel and refresh.",
            ));
        }
        inputs.push(json!({"branch_id":id,"expected_node_id":head}));
    }
    if !draft.nodes.iter().any(|node| {
        history_node_id(node) == draft.chosen
            && node["version_id"].as_str().is_some_and(|id| !id.is_empty())
    }) {
        return Err(t(
            "请选择这个文件的一个历史节点作为结果。",
            "Choose a history node of this file as the result.",
        ));
    }
    if !draft.selected.contains(&draft.survivor) {
        return Err(t(
            "请明确选择合并后保留的分支。",
            "Explicitly choose the branch to keep after merging.",
        ));
    }
    Ok(
        json!({"inputs":inputs,"chosen_node_id":draft.chosen,"surviving_branch_id":draft.survivor,"repository_id":draft.repository,"path":draft.path,"confirmed":true}),
    )
}
fn submit(state: RwSignal<State>) {
    let Some(draft) = state.with_untracked(|s| s.files.merge.clone()) else {
        return;
    };
    match request(&draft) {
        Ok(body) => review::submit(
            state,
            review::Kind::Merge,
            "POST",
            format!("/v1/file-manager/files/{}/merge", api::segment(&draft.file)),
            body,
        ),
        Err(error) => review::error(state, review::Kind::Merge, error),
    }
}
fn update(state: RwSignal<State>, f: impl FnOnce(&mut Draft)) {
    state.update(|s| {
        if let Some(draft) = s.files.merge.as_mut() {
            f(draft)
        }
    });
}
pub(super) fn node_label(node: &Value, index: usize) -> String {
    let date = node["created_at"]
        .as_str()
        .map(|value| {
            if value.contains('T') {
                format::timestamp(value)
            } else {
                format::timestamp_seconds(value)
            }
        })
        .unwrap_or_else(|| t("未记录时间", "Time not recorded"));
    let note = text(node, &["note"]);
    let descriptor = if note.is_empty() {
        match node["kind"].as_str().unwrap_or_default() {
            "rename" => t("路径变化", "Path changed"),
            "tags" => t("标签变化", "Tags changed"),
            "merge" => t("合并节点", "Merge node"),
            _ => t("文件内容", "File content"),
        }
    } else {
        note
    };
    format!("{} · {date} · {descriptor}", index + 1)
}
#[cfg(test)]
mod tests {
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
}
