use json_codec::Value;
use std::{collections::BTreeSet, sync::Arc};
use web_interface::i18n::t;

#[derive(Clone, Default)]
pub(crate) struct Model {
    pub disks: Vec<Value>,
    pub disks_loaded: bool,
    pub loading_disks: bool,
    pub slot: String,
    pub directory: String,
    pub writable: bool,
    /// Immutable server listing, shared by rows and pending picker contexts.
    pub entries: Arc<[Value]>,
    pub directory_branches: Vec<Value>,
    pub directory_file_count: u64,
    pub directory_branches_supported: bool,
    pub repository_id: String,
    pub repository_path: String,
    pub default_branch_scope: bool,
    pub loaded: bool,
    pub loading: bool,
    pub generation: u64,
    pub selected: Option<Value>,
    pub file_history: Value,
    pub history_loading: bool,
    pub history_generation: u64,
    pub checked_entries: BTreeSet<String>,
    pub batch: Option<super::batch::Draft>,
    pub merge: Option<super::merge::Draft>,
    pub backup: Option<super::backup::Draft>,
    pub tag_edit: Option<super::tags::Draft>,
    pub checked_nodes: BTreeSet<String>,
    pub publications: super::review::Publications,
    pub preview: Option<(String, String)>,
    pub preview_generation: u64,
    pub download_pending: bool,
    pub download_copy: Option<(String, bool)>,
    pub download_cleanup_pending: bool,
    pub upload_picker: Option<super::actions::PickerContext>,
    pub operations: Vec<Value>,
    pub polling_operations: Vec<String>,
    pub mutation_pending: bool,
    pub query: String,
}
impl Model {
    pub(crate) fn close_overlays(&mut self) {
        self.preview = None;
        self.preview_generation += 1;
    }

    pub(crate) fn clear_selection(&mut self) {
        self.merge = None;
        self.backup = None;
        self.tag_edit = None;
        self.checked_nodes.clear();
        self.selected = None;
        self.file_history = Value::Null;
        self.history_loading = false;
        self.history_generation += 1;
        self.close_overlays();
    }

    pub(crate) fn reconcile_selection(&mut self) {
        self.checked_entries
            .retain(|key| self.entries.iter().any(|entry| entry_key(entry) == *key));
        let Some(selected) = &self.selected else {
            return;
        };
        let identity = |entry: &Value| {
            entry["entry_id"]
                .as_str()
                .or_else(|| entry["path"].as_str())
                .unwrap_or_default()
                .to_owned()
        };
        let id = identity(selected);
        let candidates = self
            .entries
            .iter()
            .filter(|entry| identity(entry) == id)
            .collect::<Vec<_>>();
        let matched = candidates
            .iter()
            .find(|entry| entry["path"] == selected["path"])
            .copied()
            .or_else(|| {
                if candidates.len() == 1 {
                    Some(candidates[0])
                } else {
                    None
                }
            });
        if let Some(entry) = matched {
            let content_changed = ["file_id", "version_id", "node_id", "selected_branch_id"]
                .iter()
                .any(|key| selected[*key] != entry[*key]);
            let changed = content_changed || selected["path"] != entry["path"];
            let entry = entry.clone();
            if changed {
                // A refreshed branch or renamed logical path invalidates both
                // visible content and in-flight reads of the previous context.
                if content_changed {
                    self.file_history = Value::Null;
                }
                self.history_loading = false;
                self.history_generation += 1;
                self.checked_nodes.clear();
                self.close_overlays();
            }
            self.selected = Some(entry);
        } else {
            self.clear_selection();
        }
    }
}
#[derive(Clone, PartialEq)]
pub(crate) enum NamespaceAction {
    CreateDirectory,
    Rename(Value),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum SaveStatus {
    #[default]
    Waiting,
    Uploading,
    Saving,
    Unknown,
    Published,
    Degraded,
    Rejected,
}
impl SaveStatus {
    pub fn pending(self) -> bool {
        matches!(
            self,
            Self::Waiting | Self::Uploading | Self::Saving | Self::Unknown
        )
    }
    pub fn label(self) -> String {
        match self {
            Self::Waiting => t("等待上传", "Queued"),
            Self::Uploading => t("正在上传", "Uploading"),
            Self::Saving => t("正在保存", "Saving"),
            Self::Unknown => t("正在确认保存结果", "Checking save result"),
            Self::Published => t("已保存", "Saved"),
            Self::Degraded => t("已保存，保护不足", "Saved with reduced protection"),
            Self::Rejected => t("未保存", "Not saved"),
        }
    }
}
pub(crate) fn operation_status(value: &Value) -> SaveStatus {
    let value = value.get("operation").unwrap_or(value);
    let state = value["commit_state"]
        .as_str()
        .or_else(|| value["state"].as_str())
        .unwrap_or_default();
    match state {
        "PUBLISHED_DEGRADED" => SaveStatus::Degraded,
        "PUBLISHED" | "ACKED" => {
            if value["result"]["outcome"] == "COMMITTED_DEGRADED"
                || value["result"]["protection_at_publish"] == "BELOW_TARGET"
            {
                SaveStatus::Degraded
            } else {
                SaveStatus::Published
            }
        }
        "REJECTED" | "FAILED" | "CANCELLED" | "ABORTED" | "failed" | "cancelled" => {
            SaveStatus::Rejected
        }
        "SUCCEEDED" | "completed" | "succeeded" => SaveStatus::Published,
        _ => SaveStatus::Unknown,
    }
}
pub(crate) fn publication_notice(status: SaveStatus) -> String {
    match status {
        SaveStatus::Published => t("更改已保存。", "Changes saved."),
        SaveStatus::Degraded => t(
            "文件已发布，但发布时保护不足；永久损失的内容无法恢复。请检查硬盘状态。",
            "The file was published with reduced protection. Permanently lost content cannot be recovered. Check disk status.",
        ),
        SaveStatus::Rejected => t(
            "保存未完成，服务端暂存内容已保留。",
            "The save did not complete. Staged content was retained on the server.",
        ),
        _ => t(
            "保存结果尚未确认，正在查询原操作。请勿重复上传。",
            "The save is not yet confirmed. The original operation is being checked; do not upload it again.",
        ),
    }
}
pub(crate) fn join_path(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    }
}
pub(crate) fn breadcrumbs(path: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut full = String::new();
    for name in path.split('/').filter(|part| !part.is_empty()) {
        full = join_path(&full, name);
        result.push((name.to_owned(), full.clone()));
    }
    result
}
pub(crate) fn valid_name(name: &str) -> bool {
    name_policy::validate_name(name).is_ok()
}
pub(crate) fn file_kind(value: &Value) -> bool {
    value["kind"] == "file"
}
/// Readability follows the visible real version, including a pending branch's
/// inherited parent. A missing version never falls back to a source ID locally.
pub(crate) fn visible_content(value: &Value) -> bool {
    value["version_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty())
}
/// Materialization describes ownership, independently of readable inherited
/// content. Body-history records do not carry branch ownership markers.
pub(crate) fn materialized(value: &Value) -> bool {
    value["materialized"] != false
        && value["inherited"] != true
        && value["content_state"] != "PENDING"
        && visible_content(value)
}
pub(crate) fn inherited_content(value: &Value) -> bool {
    visible_content(value) && (value["inherited"] == true || value["materialized"] == false)
}
/// A stable file ID can appear at several frozen logical paths. History and
/// branch selection must carry the view path, independently of physical owner.
pub(crate) fn entry_key(entry: &Value) -> String {
    format!(
        "{}\0{}",
        entry["entry_id"].as_str().unwrap_or_default(),
        entry["path"].as_str().unwrap_or_default()
    )
}
pub(crate) fn file_context(model: &Model, entry: &Value) -> Option<(String, String)> {
    // Backup names belong to a different logical namespace. An absent primary
    // alias is authoritative, never a request to reconstruct one from its name.
    if entry["historical_only"] == true
        || entry["history_context_available"] == false
        || entry.get("repository_path").is_some_and(Value::is_null)
    {
        return None;
    }
    if entry["backup"] == true {
        let repository = entry["repository_id"]
            .as_str()
            .filter(|id| !id.is_empty())?;
        let path = entry["repository_path"]
            .as_str()
            .filter(|path| !path.is_empty())?;
        return Some((repository.to_owned(), path.to_owned()));
    }
    let repository = entry["repository_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .unwrap_or(&model.repository_id);
    let path = entry["repository_path"]
        .as_str()
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            entry["name"]
                .as_str()
                .filter(|name| !name.is_empty())
                .map(|name| join_path(&model.repository_path, name))
        })?;
    if repository.is_empty() {
        None
    } else {
        Some((repository.to_owned(), path))
    }
}

pub(crate) fn selected_entry_read_only(model: &Model) -> bool {
    model
        .selected
        .as_ref()
        .is_some_and(|entry| entry["read_only"] == true || entry["backup"] == true)
}

/// History metadata may describe a primary alias whose HEAD is newer than the
/// body physically present on the selected backup disk. Keep backup inspection
/// pinned to the exact entry while still displaying all historical nodes.
pub(crate) fn history_for_entry(mut history: Value, entry: &Value) -> Value {
    if entry["backup"] != true {
        return history;
    }
    let version = entry["version_id"].as_str().unwrap_or_default().to_owned();
    let node = entry["node_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .unwrap_or(&version)
        .to_owned();
    history["version_id"] = version.clone().into();
    history["node_id"] = node.clone().into();
    history["current_node_id"] = node.clone().into();
    history["read_only"] = true.into();
    history["inherited"] = entry["inherited"].as_bool().unwrap_or(false).into();
    history["materialized"] = entry["materialized"].as_bool().unwrap_or(true).into();
    for (field, selected) in [("nodes", &node), ("versions", &version)] {
        if let Some(records) = history[field].as_array_mut() {
            for record in records {
                let id_field = if field == "nodes" {
                    "node_id"
                } else {
                    "version_id"
                };
                record["current"] = (record[id_field].as_str() == Some(selected.as_str())).into();
            }
        }
    }
    history
}

/// Path/content ancestry is the canonical node graph, not the body-version list.
pub(crate) fn history_nodes(history: &Value) -> Vec<Value> {
    history
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
pub(crate) fn history_node_id(node: &Value) -> String {
    node["node_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .unwrap_or_default()
        .to_owned()
}
pub(crate) fn selected_branch(model: &Model) -> String {
    let selected = model.file_history["selected_branch_id"].as_str();
    model.file_history["branches"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|branch| {
            branch["current"] == true
                || branch["selected"] == true
                || selected.is_some_and(|id| branch["branch_id"] == id)
        })
        .and_then(|branch| branch["branch_id"].as_str())
        .or(selected)
        .or_else(|| {
            model
                .selected
                .as_ref()
                .and_then(|entry| entry["selected_branch_id"].as_str())
        })
        .unwrap_or_default()
        .to_owned()
}
pub(crate) fn branch_writable(branch: &Value) -> bool {
    branch["read_only"] != true
        && branch["is_alias"] != true
        && branch["alias"] != true
        && branch["alias_of"].as_str().is_none_or(str::is_empty)
        && branch["status"] != "ARCHIVED"
        && branch["status"] != "CONSUMED"
}
pub(crate) fn selected_file_read_only(model: &Model) -> bool {
    if model.file_history["read_only"] == true || selected_entry_read_only(model) {
        return true;
    }
    let id = selected_branch(model);
    model.file_history["branches"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|branch| branch["branch_id"] == id)
        .is_some_and(|branch| !branch_writable(branch))
}
pub(crate) fn selected_file_branch_record(model: &Model) -> Option<(String, String, String)> {
    let file = model
        .selected
        .as_ref()
        .and_then(|entry| entry["file_id"].as_str())
        .filter(|id| !id.is_empty())?;
    let id = selected_branch(model);
    let branch = model.file_history["branches"]
        .as_array()?
        .iter()
        .find(|branch| branch["branch_id"] == id)?;
    let name = branch["name"].as_str()?;
    if !branch_writable(branch) {
        return None;
    }
    if id.is_empty() {
        None
    } else {
        Some((file.to_owned(), id, name.to_owned()))
    }
}
pub(crate) fn directory_branch_name(value: &Value) -> String {
    match value["name"].as_str().unwrap_or_default() {
        "main" => t("主版本", "Main"),
        "" => t("未命名分支", "Unnamed branch"),
        name => name.to_owned(),
    }
}
pub(crate) fn branch_name(value: &Value) -> String {
    let name = directory_branch_name(value);
    if value["materialized"] == false || value["content_state"] == "PENDING" {
        format!("{name} · {}", t("待写入", "Pending content"))
    } else {
        name
    }
}
pub(crate) fn directory_branch_value(branch: &Value, default_scope: bool) -> String {
    branch[if default_scope {
        "directory_branch_id"
    } else {
        "name"
    }]
    .as_str()
    .unwrap_or_default()
    .to_owned()
}
pub(crate) fn selected_directory_branch(model: &Model) -> Option<(String, String, String)> {
    if model.repository_id.is_empty() {
        return None;
    }
    let branch = model
        .directory_branches
        .iter()
        .find(|branch| branch["current"] == true)?;
    let id = branch["directory_branch_id"]
        .as_str()
        .filter(|id| !id.is_empty())?;
    let name = branch["name"].as_str()?;
    Some((model.repository_id.clone(), id.to_owned(), name.to_owned()))
}

#[cfg(test)]
#[path = "../../../../test/web/unit/files/model/tests.rs"]
mod tests;
