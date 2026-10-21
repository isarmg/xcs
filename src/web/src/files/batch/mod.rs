//! Explicit, frozen branch scopes for file operations. Reviewing an operation
//! never switches the current directory or the SMB default branch.
use super::{
    actions as file_actions,
    model::{Model, entry_key, join_path, valid_name},
};
use crate::{
    api,
    ui::{State, current_session, fail, list, set_notice, text},
};
use json_codec::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use web_interface::tasks::spawn_local;
use web_interface::{i18n::t, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Move,
    Copy,
    Delete,
    Rename,
}
impl Kind {
    fn wire(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Copy => "copy",
            Self::Delete => "delete",
            Self::Rename => "rename",
        }
    }
    fn label(self) -> String {
        match self {
            Self::Move => t("移动", "Move"),
            Self::Copy => t("复制", "Copy"),
            Self::Delete => t("删除", "Delete"),
            Self::Rename => t("重命名", "Rename"),
        }
    }
    fn has_destination(self) -> bool {
        matches!(self, Self::Move | Self::Copy)
    }
}
#[derive(Clone, Default)]
pub(crate) struct Mapping {
    source: String,
    target: String,
}
#[derive(Clone)]
pub(crate) struct Draft {
    id: String,
    kind: Kind,
    repository: String,
    repository_path: String,
    disk_prefix: String,
    entries: Vec<Value>,
    names: BTreeMap<String, String>,
    sources: Vec<Value>,
    selected_sources: BTreeSet<String>,
    catalogue: Vec<Value>,
    target_path: String,
    targets: Vec<Value>,
    target_resolved: bool,
    mappings: Vec<Mapping>,
    loading: bool,
    submitted_operation: Option<String>,
    generation: u64,
    error: String,
}
fn branch_id(branch: &Value) -> String {
    text(branch, &["directory_branch_id"])
}
fn branch_label(branch: &Value) -> String {
    text(branch, &["name"])
}
fn entry_id(entry: &Value) -> String {
    text(entry, &["entry_id"])
}
fn source_branches(model: &Model) -> Vec<Value> {
    model.directory_branches.clone()
}

pub(crate) fn available(model: &Model) -> bool {
    model.writable
        && model.loaded
        && !model.loading
        && !model.repository_id.is_empty()
        && (!model.checked_entries.is_empty() || model.selected.is_some())
}

mod actions;
mod request;
mod view;
pub(crate) use actions::start;
use request::{request, valid_path};
pub(crate) use view::view;

/// Keep an uncertain transaction tied to its original operation. A later poll
/// closes only that draft, so retrying cannot duplicate a completed copy.
pub(crate) fn settle_operation(model: &mut Model, id: &str, status: super::model::SaveStatus) {
    if !model
        .batch
        .as_ref()
        .is_some_and(|draft| draft.submitted_operation.as_deref() == Some(id))
    {
        return;
    }
    match status {
        super::model::SaveStatus::Published | super::model::SaveStatus::Degraded => {
            model.batch = None;
            model.checked_entries.clear();
        }
        super::model::SaveStatus::Rejected => {
            if let Some(draft) = model.batch.as_mut() {
                draft.submitted_operation = None;
            }
        }
        _ => {}
    }
}
