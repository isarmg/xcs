use serde_json::Value;
use std::collections::BTreeSet;
use web_interface::i18n::t;

#[derive(Clone, Default)]
pub(crate) struct Model {
    pub disks: Vec<Value>,
    pub disks_loaded: bool,
    pub loading_disks: bool,
    pub slot: String,
    pub directory: String,
    pub writable: bool,
    pub entries: Vec<Value>,
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
mod tests {
    use super::*;

    #[test]
    fn refreshed_selection_invalidates_old_history_and_preview_only_when_context_changes() {
        let original = serde_json::json!({"entry_id":"entry", "file_id":"file", "path":"a.txt", "version_id":"v1", "selected_branch_id":"main"});
        let mut model = Model {
            selected: Some(original.clone()),
            entries: vec![original.clone()],
            file_history: serde_json::json!({"selected_branch_id":"main"}),
            preview: Some(("a.txt".into(), "old content".into())),
            history_loading: true,
            ..Default::default()
        };
        model.reconcile_selection();
        assert!(model.preview.is_some());
        assert_eq!(model.history_generation, 0);
        model.entries[0]["selected_branch_id"] = "draft".into();
        model.entries[0]["version_id"] = "v2".into();
        model.checked_nodes.insert("old-node".into());
        model.reconcile_selection();
        assert_eq!(model.selected.as_ref().unwrap()["version_id"], "v2");
        assert!(model.preview.is_none());
        assert!(model.file_history.is_null());
        assert!(!model.history_loading);
        assert!(model.checked_nodes.is_empty());
        assert_eq!(model.history_generation, 1);
        assert_eq!(model.preview_generation, 1);
    }
    #[test]
    fn backup_context_never_guesses_a_primary_alias_from_its_prefix() {
        let model = Model {
            repository_id: "wrong-listing-repo".into(),
            repository_path: "wrong-prefix".into(),
            ..Default::default()
        };
        let mut backup = serde_json::json!({"backup":true,"read_only":true,"name":"f.txt","path":"我的备份/f.txt","repository_id":"repo","repository_path":"actual/primary/f.txt"});
        assert_eq!(
            file_context(&model, &backup),
            Some(("repo".into(), "actual/primary/f.txt".into()))
        );
        backup["repository_path"] = Value::Null;
        assert!(file_context(&model, &backup).is_none());
        backup["repository_path"] = "stale/alias.txt".into();
        backup["historical_only"] = true.into();
        assert!(file_context(&model, &backup).is_none());
        backup.as_object_mut().unwrap().remove("historical_only");
        backup.as_object_mut().unwrap().remove("repository_path");
        assert!(file_context(&model, &backup).is_none());
    }

    #[test]
    fn backup_history_keeps_its_stored_body_and_current_node() {
        let entry = serde_json::json!({"backup":true,"read_only":true,"version_id":"stored-old","node_id":"stored-old"});
        let history = serde_json::json!({"version_id":"primary-new","node_id":"primary-new","nodes":[{"node_id":"stored-old","version_id":"stored-old","current":false},{"node_id":"primary-new","version_id":"primary-new","current":true}],"versions":[{"version_id":"stored-old"},{"version_id":"primary-new","current":true}]});
        let result = history_for_entry(history.clone(), &entry);
        assert_eq!(result["version_id"], "stored-old");
        assert_eq!(result["current_node_id"], "stored-old");
        assert_eq!(result["nodes"][0]["current"], true);
        assert_eq!(result["nodes"][1]["current"], false);
        assert_eq!(result["versions"][0]["current"], true);
        assert_eq!(result["read_only"], true);
        assert_eq!(
            history_for_entry(
                history.clone(),
                &serde_json::json!({"version_id":"stored-old"})
            ),
            history
        );
        let mut model = Model {
            writable: true,
            selected: Some(entry),
            file_history: serde_json::json!({"branches":[{"branch_id":"main","current":true}]}),
            ..Default::default()
        };
        assert!(selected_entry_read_only(&model));
        assert!(selected_file_read_only(&model));
        model.selected = Some(serde_json::json!({"read_only":true}));
        assert!(selected_entry_read_only(&model));
    }
    const DEGRADED: &str = include_str!("fixtures/degraded-save-operation.json");
    const DAMAGE: &str = include_str!("fixtures/damaged-published-save-operation.json");
    #[test]
    fn publication_protection_is_not_rewritten_by_later_damage() {
        let damage: Value = serde_json::from_str(DAMAGE).unwrap();
        // Derive a healthy comparison from this real receipt, changing only
        // current health. Frozen publication evidence must give the same label.
        let mut good = damage.clone();
        good["current_health"]["state"] = "HEALTHY".into();
        assert_eq!(good["result"], damage["result"]);
        assert_eq!(operation_status(&good), SaveStatus::Published);
        assert_eq!(operation_status(&damage), SaveStatus::Published);
    }
    #[test]
    fn permanent_loss_is_distinct_from_ordinary_save() {
        let loss: Value = serde_json::from_str(DEGRADED).unwrap();
        assert_eq!(operation_status(&loss), SaveStatus::Degraded);
        assert!(publication_notice(SaveStatus::Degraded).contains("Permanently lost"));
    }
    #[test]
    fn uncertain_save_cannot_be_relabelled_as_a_failure() {
        assert_eq!(
            operation_status(&serde_json::json!({"state":"in_doubt"})),
            SaveStatus::Unknown
        );
        assert!(SaveStatus::Unknown.pending());
        assert!(!SaveStatus::Rejected.pending());
    }
    #[test]
    fn breadcrumbs_stay_relative_to_the_selected_disk() {
        assert_eq!(
            breadcrumbs("photos/trips"),
            vec![
                ("photos".into(), "photos".into()),
                ("trips".into(), "photos/trips".into())
            ]
        );
        assert_eq!(join_path("", "photo.txt"), "photo.txt");
    }
    #[test]
    fn names_cannot_escape_the_directory() {
        for bad in ["", ".", "..", "a/b", "a\\b", "a\n"] {
            assert!(!valid_name(bad));
        }
        assert!(valid_name("report 2026.txt"));
    }
    #[test]
    fn visible_inherited_content_is_readable_without_becoming_owned_content() {
        let pending = serde_json::json!({"materialized":false,"content_state":"PENDING","version_id":null,"source_version_id":"source"});
        assert!(!materialized(&pending));
        assert!(!visible_content(&pending));
        let inherited = serde_json::json!({"materialized":false,"inherited":true,"version_id":"source","own_version_id":null,"source_version_id":"source","size":"6"});
        assert!(visible_content(&inherited));
        assert!(inherited_content(&inherited));
        assert!(!materialized(&inherited));
        assert!(!materialized(
            &serde_json::json!({"version_id":null,"source_version_id":"source"})
        ));
        assert!(!materialized(
            &serde_json::json!({"materialized":false,"version_id":"source"})
        ));
        assert!(!materialized(
            &serde_json::json!({"materialized":true,"version_id":null,"source_version_id":"source"})
        ));
        assert!(materialized(&serde_json::json!({"version_id":"body"})));
        assert!(materialized(
            &serde_json::json!({"materialized":true,"version_id":"saved","size":"0"})
        ));
        assert!(!inherited_content(
            &serde_json::json!({"materialized":true,"inherited":false,"version_id":"saved","own_version_id":"saved","size":"0"})
        ));
        assert!(
            branch_name(&serde_json::json!({"name":"draft","materialized":false}))
                .contains("Pending content")
        );
        assert_eq!(branch_name(&serde_json::json!({"name":"legacy"})), "legacy");
    }
    #[test]
    fn current_branch_identity_does_not_require_a_content_version() {
        let mut model = Model {
            file_history: serde_json::json!({"selected_branch_id":"pending","branches":[{"branch_id":"pending","materialized":false,"version_id":null}]}),
            ..Default::default()
        };
        assert_eq!(selected_branch(&model), "pending");
        model.file_history = Value::Null;
        model.selected = Some(serde_json::json!({"selected_branch_id":"entry-pending"}));
        assert_eq!(selected_branch(&model), "entry-pending");
    }
    #[test]
    fn an_authoritative_history_node_list_does_not_duplicate_its_body_versions() {
        let mut history = serde_json::json!({"versions":[{"version_id":"ver-one"}],"nodes":[{"node_id":"fnode-rename","version_id":"ver-one"}]});
        assert_eq!(history_nodes(&history).len(), 1);
        assert_eq!(history_node_id(&history_nodes(&history)[0]), "fnode-rename");
        history["nodes"] = serde_json::json!([]);
        assert!(history_nodes(&history).is_empty());
        history.as_object_mut().unwrap().remove("nodes");
        assert!(history_nodes(&history).is_empty());
        assert!(history_node_id(&serde_json::json!({"version_id":"ver-one"})).is_empty());
    }
    #[test]
    fn generated_public_file_branch_names_are_editable_only_after_loading_a_real_record() {
        let mut model = Model {
            selected: Some(
                serde_json::json!({"file_id":"file-a","selected_branch_id":"generated-a"}),
            ),
            ..Default::default()
        };
        assert!(selected_file_branch_record(&model).is_none());
        model.file_history = serde_json::json!({"selected_branch_id":"generated-a","branches":[{"branch_id":"generated-a","name":"方案甲-7c2a","materialized":true},{"branch_id":"other","name":"other"}]});
        assert_eq!(
            selected_file_branch_record(&model),
            Some(("file-a".into(), "generated-a".into(), "方案甲-7c2a".into()))
        );
        model.file_history["branches"][0]["name"] = "用户改名".into();
        assert_eq!(
            selected_file_branch_record(&model).unwrap().1,
            "generated-a"
        );
        assert_eq!(selected_file_branch_record(&model).unwrap().2, "用户改名");
    }
    #[test]
    fn root_default_selection_uses_real_directory_ids_and_subfolders_keep_names() {
        let branch = serde_json::json!({"directory_branch_id":"directory-a","name":"auto-generated","current":true,"materialized":false});
        assert_eq!(directory_branch_value(&branch, true), "directory-a");
        assert_eq!(directory_branch_value(&branch, false), "auto-generated");
        assert_eq!(
            directory_branch_name(&branch),
            "auto-generated",
            "directory records never claim to own pending file content"
        );
        let mut model = Model {
            repository_id: "repository-a".into(),
            directory_branches: vec![branch],
            ..Default::default()
        };
        assert_eq!(
            selected_directory_branch(&model),
            Some((
                "repository-a".into(),
                "directory-a".into(),
                "auto-generated".into()
            ))
        );
        model.directory_branches.clear();
        assert!(
            selected_directory_branch(&model).is_none(),
            "initial namespace main has no invented rename target"
        );
        model
            .directory_branches
            .push(serde_json::json!({"name":"main","current":true,"directory_branch_id":null}));
        assert!(selected_directory_branch(&model).is_none());
    }
    #[test]
    fn renaming_an_entry_preserves_selection_and_file_history() {
        let mut model = Model {
            selected: Some(
                serde_json::json!({"entry_id":"file-a","name":"old.txt","path":"old.txt"}),
            ),
            entries: vec![
                serde_json::json!({"entry_id":"file-a","name":"new.txt","path":"new.txt"}),
            ],
            file_history: serde_json::json!({"versions":[{"version_id":"v1"}]}),
            ..Default::default()
        };
        model.reconcile_selection();
        assert_eq!(model.selected.unwrap()["name"], "new.txt");
        assert_eq!(model.file_history["versions"][0]["version_id"], "v1");
        assert_eq!(
            model.history_generation, 1,
            "old path responses are invalidated"
        );
    }
    #[test]
    fn file_context_uses_the_logical_alias_instead_of_the_container_owner() {
        let model = Model {
            repository_id: "repo".into(),
            repository_path: "a".into(),
            ..Default::default()
        };
        let alias = serde_json::json!({"entry_id":"f","file_id":"f","name":"old.txt","path":"disk/a/old.txt","repository_id":"repo","repository_path":"a/old.txt","container_path":"b/new.txt"});
        assert_eq!(
            file_context(&model, &alias),
            Some(("repo".into(), "a/old.txt".into()))
        );
        assert_eq!(
            file_context(&model, &serde_json::json!({"name":"fallback.txt"})),
            Some(("repo".into(), "a/fallback.txt".into()))
        );
        assert_ne!(
            entry_key(&alias),
            entry_key(&serde_json::json!({"entry_id":"f","path":"b/new.txt"}))
        );
    }
    #[test]
    fn two_aliases_of_one_file_keep_the_selected_logical_path() {
        let mut model = Model {
            selected: Some(serde_json::json!({"entry_id":"f","path":"b/f.txt"})),
            entries: vec![
                serde_json::json!({"entry_id":"f","path":"a/f.txt","version_id":"F1"}),
                serde_json::json!({"entry_id":"f","path":"b/f.txt","version_id":"F2"}),
            ],
            ..Default::default()
        };
        model.reconcile_selection();
        assert_eq!(model.selected.as_ref().unwrap()["version_id"], "F2");
        model.selected.as_mut().unwrap()["path"] = "missing/f.txt".into();
        model.reconcile_selection();
        assert!(model.selected.is_none());
    }
    #[test]
    fn a_removed_entry_discards_metadata_and_pending_preview() {
        let mut model = Model {
            selected: Some(serde_json::json!({"entry_id":"file-a"})),
            file_history: serde_json::json!({"versions":[{"version_id":"v1"}]}),
            history_loading: true,
            history_generation: 4,
            preview: Some(("old.txt".into(), "old content".into())),
            preview_generation: 2,

            ..Default::default()
        };
        model.reconcile_selection();
        assert!(model.selected.is_none());
        assert!(model.file_history.is_null());
        assert!(!model.history_loading);
        assert_eq!(model.history_generation, 5);
        assert_eq!(model.preview_generation, 3);
        assert!(model.preview.is_none());
    }
}
