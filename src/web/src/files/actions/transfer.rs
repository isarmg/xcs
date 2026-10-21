//! Sequential uploads, verified content reads and downloads.
use super::{browse::load_entries, disk_path, write_allowed};
use crate::{
    api,
    files::model::{
        Model, SaveStatus, file_kind, join_path, operation_status, publication_notice,
        selected_branch, visible_content,
    },
    transfers::UploadRow,
    ui::{State, current_session, fail, set_notice, text},
};
use serde_json::{Value, json};
use web_interface::tasks::spawn_local;
use web_interface::{i18n::t, prelude::*};

// Capture the destination before the browser file picker opens. Navigation or a
// branch/session change invalidates a late result instead of retargeting it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PickerContext {
    epoch: u64,
    generation: u64,
    history_generation: u64,
    slot: String,
    directory: String,
    selected: Option<Value>,
    branch: String,
    entries: Vec<Value>,
    directory_branches: Vec<Value>,
    replace: bool,
}
impl PickerContext {
    fn capture(state: &State, replace: bool) -> Self {
        let model = &state.files;
        Self {
            epoch: state.session_epoch,
            generation: model.generation,
            history_generation: model.history_generation,
            slot: model.slot.clone(),
            directory: model.directory.clone(),
            selected: model.selected.clone(),
            branch: selected_branch(model),
            entries: model.entries.clone(),
            directory_branches: model.directory_branches.clone(),
            replace,
        }
    }
    fn current(&self, state: &State) -> bool {
        state.session_active() && *self == Self::capture(state, self.replace)
    }
}
/// Returns true only when the view should open its browser file input.
pub(crate) fn pick_upload(state: RwSignal<State>, replace: bool) -> bool {
    let snapshot = state.get_untracked();
    if !write_allowed(&snapshot) || (replace && snapshot.files.history_loading) {
        return false;
    }
    let context = PickerContext::capture(&snapshot, replace);
    state.update(|s| s.files.upload_picker = Some(context));
    true
}
pub(crate) fn cancel_picker(state: RwSignal<State>) {
    state.update(|s| s.files.upload_picker = None);
}
fn take_picker_context(state: RwSignal<State>) -> Option<PickerContext> {
    let context = state.with_untracked(|s| s.files.upload_picker.clone());
    cancel_picker(state);
    context
}
pub(crate) fn upload_files(state: RwSignal<State>, files: Option<web_sys::FileList>) {
    let Some(context) = take_picker_context(state) else {
        return;
    };
    let sources = files
        .into_iter()
        .flat_map(|files| (0..files.length()).filter_map(move |i| files.get(i)))
        .map(api::UploadSource::from)
        .collect();
    queue_uploads(state, sources, context);
}

#[derive(Debug, PartialEq, Eq)]
struct UploadTarget {
    name: String,
    path: String,
    branch_id: String,
}
fn upload_target(model: &Model, local_name: &str, replace_selected: bool) -> Option<UploadTarget> {
    if replace_selected {
        if crate::files::model::selected_entry_read_only(model) {
            return None;
        }
        let entry = model.selected.as_ref().filter(|entry| file_kind(entry))?;
        let name = text(entry, &["name"]);
        let path = text(entry, &["path"]);
        if name.is_empty() || path.is_empty() {
            return None;
        }
        return Some(UploadTarget {
            name,
            path,
            branch_id: selected_branch(model),
        });
    }
    if !crate::files::model::valid_name(local_name) {
        return None;
    }
    let path = join_path(&model.directory, local_name);
    let branch_id = if model
        .selected
        .as_ref()
        .is_some_and(|entry| text(entry, &["path"]) == path)
    {
        selected_branch(model)
    } else {
        model
            .entries
            .iter()
            .find(|entry| file_kind(entry) && text(entry, &["path"]) == path)
            .map(|entry| text(entry, &["selected_branch_id"]))
            .unwrap_or_default()
    };
    Some(UploadTarget {
        name: local_name.to_owned(),
        path,
        branch_id,
    })
}
fn write_session_body(row: &UploadRow) -> Value {
    let mut request = json!({"path":row.path});
    if !row.branch_id.is_empty() {
        request["branch_id"] = row.branch_id.clone().into();
    }
    request
}
fn record_preflight_failure(
    state: RwSignal<State>,
    model: &Model,
    source: &api::UploadSource,
    target: Option<UploadTarget>,
    message: String,
) {
    let Some(row) = preflight_failure(
        model,
        source.name(),
        target,
        source.size(),
        api::operation_id(),
        message,
    ) else {
        // The caller still displays the preflight error. A missing strong ID
        // must not create duplicate ledger keys or a weak server operation ID.
        return;
    };
    state.update(|s| s.transfers.rows.push(row));
}
fn preflight_failure(
    model: &Model,
    name: String,
    target: Option<UploadTarget>,
    size: Option<u64>,
    id: String,
    message: String,
) -> Option<UploadRow> {
    if id.is_empty() {
        return None;
    }
    let mut row = UploadRow {
        operation_id: id,
        path: target
            .as_ref()
            .map(|target| target.path.clone())
            .unwrap_or_default(),
        branch_id: target
            .as_ref()
            .map(|target| target.branch_id.clone())
            .unwrap_or_default(),
        name,
        slot: model.slot.clone(),
        size,
        status: SaveStatus::Rejected,
        message: format!(
            "{} {}",
            message,
            t(
                "未发送文件或保存操作。请检查后重新选择。",
                "No file or save operation was sent. Check the file and select it again."
            )
        ),
        ..Default::default()
    };
    row.settle_time();
    Some(row)
}

fn queue_uploads(state: RwSignal<State>, sources: Vec<api::UploadSource>, context: PickerContext) {
    let snapshot = state.get_untracked();
    if !context.current(&snapshot) || !write_allowed(&snapshot) {
        return;
    }
    if sources.is_empty() {
        return;
    }
    if context.replace && (sources.len() != 1 || snapshot.files.history_loading) {
        return;
    }
    if sources.len() > 128 {
        set_notice(
            state,
            t(
                "每次最多选择 128 个文件。",
                "Select no more than 128 files at once.",
            ),
        );
        return;
    }
    let mut queue = Vec::new();
    for source in sources {
        // Validate the entire queue before creating any server workspace.
        let Some(target) = upload_target(&snapshot.files, &source.name(), context.replace) else {
            let message = t(
                "上传文件名或目标无效，本文件未上传。",
                "The upload filename or destination is invalid. This file was not uploaded.",
            );
            record_preflight_failure(state, &snapshot.files, &source, None, message.clone());
            set_notice(state, message);
            return;
        };
        if let Err(error) = source.preflight() {
            record_preflight_failure(
                state,
                &snapshot.files,
                &source,
                Some(target),
                error.to_string(),
            );
            fail(state, error);
            return;
        }
        let id = api::operation_id();
        if id.is_empty() {
            return;
        }
        let size = source.size();
        queue.push((
            source,
            UploadRow {
                operation_id: id,
                size,
                name: target.name,
                slot: snapshot.files.slot.clone(),
                path: target.path,
                branch_id: target.branch_id,
                ..Default::default()
            },
        ));
    }
    state.update(|s| {
        append_uploads(
            &mut s.transfers.rows,
            queue.iter().map(|(_, row)| row.clone()),
        );
    });
    let epoch = snapshot.session_epoch;
    let task = async move {
        for (source, row) in queue {
            if !current_session(state, epoch) {
                return;
            }
            let id = row.operation_id.clone();
            update_upload(state, &id, |row| {
                row.status = SaveStatus::Uploading;
                row.started_at = crate::transfers::now();
            });
            let request = write_session_body(&row);
            let mut commit_attempted = false;
            let result = async {
                let session = api::request(
                    "POST",
                    &format!("{}/write-sessions", disk_path(&row.slot)),
                    Some(request),
                )
                .await?;
                if !current_session(state, epoch) {
                    return Err(api::session_changed());
                }
                let workspace = text(&session, &["workspace_id"]);
                let expected_head = text(&session, &["base_head"]);
                let path = text(&session, &["relative_path"]);
                if workspace.is_empty() || path.is_empty() {
                    return Err(api::invalid_response());
                }
                update_upload(state, &id, |row| {
                    row.workspace = workspace.clone();
                    row.expected_head = expected_head.clone();
                });
                api::upload(&workspace, &api::entry_id(), &path, source).await?;
                if !current_session(state, epoch) {
                    return Err(api::session_changed());
                }
                update_upload(state, &id, |row| row.status = SaveStatus::Saving);
                api::commit_workspace(
                    &workspace,
                    &id,
                    &format!("Upload {}", row.name),
                    &expected_head,
                    || {
                        commit_attempted = true;
                        state.try_update(|s| {
                            if let Some(row) = s
                                .transfers
                                .rows
                                .iter_mut()
                                .find(|row| row.operation_id == id)
                            {
                                row.commit_attempted = true;
                            }
                        });
                    },
                )
                .await
            }
            .await;
            if !current_session(state, epoch) {
                return;
            }
            match result {
                Ok(value) => apply_upload_result(state, &id, value),
                Err(error) => {
                    update_upload(state, &id, |row| {
                        apply_upload_error(row, commit_attempted, &error);
                        if !commit_attempted {
                            row.message.push_str(&t(
                                " 请重新选择文件后重试。",
                                " Select the file again before retrying.",
                            ));
                        }
                    });
                    fail(state, error);
                }
            }
            if state.with_untracked(|s| {
                s.transfers
                    .rows
                    .iter()
                    .find(|row| row.operation_id == id)
                    .is_some_and(|row| row.status == SaveStatus::Unknown)
            }) {
                state.update(|s| stop_waiting_uploads(&mut s.transfers.rows));
                break;
            }
        }
    };
    spawn_local(task);
}

// Saving includes sealing, but an operation exists only after commit is sent.
// A malformed/truncated successful HTTP response is also unconfirmed: the
// server may already have published it. Only a definite client error rejects it.
fn upload_error_status(commit_attempted: bool, http_status: u16) -> SaveStatus {
    if commit_attempted && !(400..500).contains(&http_status) {
        SaveStatus::Unknown
    } else {
        SaveStatus::Rejected
    }
}

fn apply_upload_error(row: &mut UploadRow, commit_attempted: bool, error: &api::Error) {
    row.status = upload_error_status(commit_attempted, error.status);
    row.message = error.to_string();
    row.request_id = error.request_id.clone();
    row.settle_time();
}

// Receipts (especially failed staged uploads) remain until the user explicitly
// clears them. Starting another upload must not erase recovery information.
fn append_uploads(uploads: &mut Vec<UploadRow>, rows: impl IntoIterator<Item = UploadRow>) {
    uploads.extend(rows);
}
/// A lost save response stops the queue, without rewriting the original
/// operation's unknown state or any file that has already been published.
fn stop_waiting_uploads(uploads: &mut [UploadRow]) {
    for row in uploads
        .iter_mut()
        .filter(|row| row.status == SaveStatus::Waiting)
    {
        row.status = SaveStatus::Rejected;
        row.settle_time();
        row.message = t(
            "前一项保存尚未确认，本文件未上传。确认后请重新选择。",
            "The previous save is unconfirmed. This file was not uploaded; select it again after confirmation.",
        );
    }
}
pub(super) fn update_upload(state: RwSignal<State>, id: &str, update: impl FnOnce(&mut UploadRow)) {
    state.update(|s| {
        if let Some(row) = s
            .transfers
            .rows
            .iter_mut()
            .find(|row| row.operation_id == id)
        {
            update(row);
        }
    });
}
pub(super) fn apply_upload_result(state: RwSignal<State>, id: &str, value: Value) {
    let value = super::operation::receipt(id, value);
    let status = operation_status(&value);
    update_upload(state, id, |row| {
        row.status = status;
        row.message = publication_notice(status);
        let item = value.get("operation").unwrap_or(&value);
        if let Some(error) = item.get("error").filter(|error| !error.is_null()) {
            let detail = error
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| error.to_string());
            row.message.push_str(&format!(" {detail}"));
        }
        row.polling = false;
        row.settle_time();
        if matches!(status, SaveStatus::Published | SaveStatus::Degraded) {
            row.workspace.clear();
            row.expected_head.clear();
        }
    });
    set_notice(state, publication_notice(status));
    if matches!(status, SaveStatus::Published | SaveStatus::Degraded) {
        load_entries(state);
    }
}
pub(crate) fn preview(state: RwSignal<State>, entry: Value) {
    if !state.with_untracked(State::session_active) || !visible_content(&entry) {
        return;
    }
    let version = text(&entry, &["version_id"]);
    let name = text(&entry, &["name"]);
    let epoch = state.with_untracked(|s| s.session_epoch);
    let generation = state.with_untracked(|s| s.files.preview_generation + 1);
    state.update(|s| s.files.preview_generation = generation);
    spawn_local(async move {
        let result = api::preview_version(&version).await;
        if !current_session(state, epoch)
            || !state
                .with_untracked(|s| s.page == "details" && s.files.preview_generation == generation)
        {
            return;
        }
        match result {
            Ok(content) => state.update(|s| s.files.preview = Some((name, content))),
            Err(error) => fail(state, error),
        }
    });
}
pub(crate) fn download(state: RwSignal<State>, entry: Value) {
    if !state.with_untracked(State::session_active) || !visible_content(&entry) {
        return;
    }
    let version = text(&entry, &["version_id"]);
    let name = text(&entry, &["name"]);
    let epoch = state.with_untracked(|s| s.session_epoch);
    spawn_local(async move {
        let result = api::download(
            &format!("/api/versions/{}/content", api::segment(&version)),
            &name,
        )
        .await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(message) => set_notice(state, message),
            Err(error) => fail(state, error),
        }
    });
}

#[cfg(test)]
#[path = "../../../../../test/web/unit/files/actions/transfer/tests.rs"]
mod tests;
