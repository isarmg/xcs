//! Sequential uploads, verified content reads and downloads.
use super::{browse::load_entries, disk_path, pending};
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
use zxs_web::tasks::spawn_local;
use zxs_web::{i18n::t, prelude::*};

// Capture the destination before either system picker opens. Navigation or a
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
        state.authenticated && !state.logout_pending && *self == Self::capture(state, self.replace)
    }
}
/// Returns true only when the view should open its browser file input.
pub(crate) fn pick_upload(state: RwSignal<State>, replace: bool) -> bool {
    let snapshot = state.get_untracked();
    if !snapshot.authenticated
        || snapshot.logout_pending
        || !snapshot.files.writable
        || snapshot.files.loading
        || pending(&snapshot)
        || (replace && snapshot.files.history_loading)
    {
        return false;
    }
    let context = PickerContext::capture(&snapshot, replace);
    state.update(|s| s.files.upload_picker = Some(context.clone()));
    if !api::native() {
        return true;
    }
    // Consume late native selections so unused one-shot handles are released,
    // even after root cleanup. Never read disposed UI state on completion.
    wasm_bindgen_futures::spawn_local(async move {
        let result = api::choose_files(!replace).await;
        let active = state
            .try_with_untracked(|s| s.files.upload_picker.as_ref() == Some(&context))
            .unwrap_or(false);
        if active {
            state.update(|s| s.files.upload_picker = None);
        }
        if !active || !state.with_untracked(|s| context.current(s)) {
            return;
        }
        match result {
            Ok(sources) => queue_uploads(state, sources, context),
            Err(error) => fail(state, error),
        }
    });
    false
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
        .map(api::UploadSource::Browser)
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
    let row = preflight_failure(
        model,
        source.name(),
        target,
        source.size(),
        api::operation_id(),
        message,
    );
    state.update(|s| s.transfers.rows.push(row));
}
fn preflight_failure(
    model: &Model,
    name: String,
    target: Option<UploadTarget>,
    size: Option<u64>,
    id: String,
    message: String,
) -> UploadRow {
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
    row
}

fn queue_uploads(state: RwSignal<State>, sources: Vec<api::UploadSource>, context: PickerContext) {
    let snapshot = state.get_untracked();
    if !context.current(&snapshot)
        || !snapshot.files.writable
        || snapshot.files.loading
        || pending(&snapshot)
    {
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
    if api::native() {
        // Keep an in-flight one-shot source alive until the host consumes it.
        // Every await is followed by a disposal-safe current_session check;
        // the commit receipt callback also uses a non-panicking signal update.
        wasm_bindgen_futures::spawn_local(task);
    } else {
        spawn_local(task);
    }
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
    if !visible_content(&entry) {
        return;
    }
    let version = text(&entry, &["version_id"]);
    let name = text(&entry, &["name"]);
    let epoch = state.with_untracked(|s| s.session_epoch);
    let generation = state.with_untracked(|s| s.files.preview_generation + 1);
    state.update(|s| s.files.preview_generation = generation);
    spawn_local(async move {
        let result = api::open_version(&version, &name).await;
        if !current_session(state, epoch)
            || !state
                .with_untracked(|s| s.page == "details" && s.files.preview_generation == generation)
        {
            return;
        }
        match result {
            Ok(Some(content)) => state.update(|s| s.files.preview = Some((name, content))),
            Ok(None) => {}
            Err(error) => fail(state, error),
        }
    });
}
pub(crate) fn download(state: RwSignal<State>, entry: Value) {
    if !visible_content(&entry) {
        return;
    }
    let version = text(&entry, &["version_id"]);
    let name = text(&entry, &["name"]);
    let epoch = state.with_untracked(|s| s.session_epoch);
    spawn_local(async move {
        let result = api::download(
            &format!("/v1/versions/{}/content", api::segment(&version)),
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
mod tests {
    use super::*;

    #[test]
    fn preflight_failure_records_metadata_without_creating_a_pollable_operation() {
        let model = Model {
            slot: "P01".into(),
            directory: "reports".into(),
            ..Default::default()
        };
        let row = preflight_failure(
            &model,
            "large.bin".into(),
            upload_target(&model, "large.bin", false),
            Some(67_108_865),
            "local-id".into(),
            "File exceeds browser limit".into(),
        );
        assert_eq!(row.status, SaveStatus::Rejected);
        assert_eq!(row.size, Some(67_108_865));
        assert_eq!(row.path, "reports/large.bin");
        assert!(row.message.contains("No file or save operation was sent"));
        assert!(row.workspace.is_empty());
        assert!(row.started_at.is_none());
        assert!(!row.commit_attempted);
        assert!(!row.queryable());
    }
    #[test]
    fn failed_replacement_keeps_actual_target_and_branch_not_local_source_name() {
        let model = pending_model();
        let row = preflight_failure(
            &model,
            "large.bin".into(),
            upload_target(&model, "large.bin", true),
            Some(67_108_865),
            "local".into(),
            "Too large".into(),
        );
        assert_eq!(row.name, "large.bin");
        assert_eq!(row.path, "reports/report.txt");
        assert_eq!(row.branch_id, "draft");
        let invalid = preflight_failure(
            &model,
            "../invalid".into(),
            None,
            None,
            "local".into(),
            "Invalid destination".into(),
        );
        assert!(invalid.path.is_empty());
        assert!(invalid.branch_id.is_empty());
    }
    #[test]
    fn cancelled_picker_does_not_retarget_a_late_browser_change() {
        let owner = Owner::new();
        owner.with(|| {
            let state = RwSignal::new(State {
                authenticated: true,
                session_epoch: 3,
                files: Model {
                    writable: true,
                    slot: "P01".into(),
                    ..Default::default()
                },
                ..Default::default()
            });
            let original = state.with_untracked(|s| PickerContext::capture(s, false));
            state.update(|s| s.files.upload_picker = Some(original));
            cancel_picker(state);
            state.update(|s| s.files.directory = "new-destination".into());
            // A missing context is not reconstructed from the new directory,
            // even when it is otherwise authenticated and writable.
            assert!(take_picker_context(state).is_none());
            upload_files(state, None);
            state.with_untracked(|s| {
                assert!(s.files.upload_picker.is_none());
                assert!(s.transfers.rows.is_empty());
            });
        });
    }

    #[test]
    fn picker_result_cannot_move_to_a_new_session_location_or_branch() {
        let state = State {
            authenticated: true,
            session_epoch: 5,
            files: Model {
                slot: "disk".into(),
                directory: "folder".into(),
                selected: Some(
                    json!({"name":"file","path":"folder/file","selected_branch_id":"main"}),
                ),
                ..Default::default()
            },
            ..Default::default()
        };
        let context = PickerContext::capture(&state, true);
        assert!(context.current(&state));
        let mut changed = state.clone();
        changed.session_epoch += 1;
        assert!(!context.current(&changed));
        let mut changed = state.clone();
        changed.files.directory = "another".into();
        assert!(!context.current(&changed));
        let mut changed = state.clone();
        changed.files.slot = "another".into();
        assert!(!context.current(&changed));
        let mut changed = state.clone();
        changed.files.selected.as_mut().unwrap()["selected_branch_id"] = "other".into();
        assert!(!context.current(&changed));
        let mut changed = state.clone();
        changed.logout_pending = true;
        assert!(!context.current(&changed));
        let mut changed = state.clone();
        changed.files.history_generation += 1;
        assert!(!context.current(&changed));
    }

    #[test]
    fn unrelated_upload_receipts_keep_recovery_data_and_stop_the_queue() {
        Owner::new().with(|| {
            for response in [
                json!({"state":"SUCCEEDED"}),
                json!({"operation_id":"other","state":"SUCCEEDED"}),
                json!({"operation":{"operation_id":"other","state":"SUCCEEDED"}}),
            ] {
                let state = RwSignal::new(State::default());
                state.update(|s| {
                    s.transfers.rows.push(UploadRow {
                        operation_id: "original".into(),
                        workspace: "retained-workspace".into(),
                        expected_head: "retained-head".into(),
                        status: SaveStatus::Saving,
                        polling: true,
                        ..Default::default()
                    })
                });
                apply_upload_result(state, "original", response);
                state.with_untracked(|s| {
                    let row = &s.transfers.rows[0];
                    assert_eq!(row.status, SaveStatus::Unknown);
                    assert!(pending(s));
                    assert!(!row.polling);
                    assert_eq!(row.workspace, "retained-workspace");
                    assert_eq!(row.expected_head, "retained-head");
                });
                apply_upload_result(
                    state,
                    "original",
                    json!({
                        "operation":{"operation_id":"original","state":"SUCCEEDED"}
                    }),
                );
                state.with_untracked(|s| {
                    let row = &s.transfers.rows[0];
                    assert_eq!(row.status, SaveStatus::Published);
                    assert!(!pending(s));
                    assert!(row.workspace.is_empty());
                    assert!(row.expected_head.is_empty());
                });
            }
        });
    }

    #[test]
    fn seal_failures_do_not_poll_an_operation_that_was_never_submitted() {
        // Uploading has finished and the row says Saving while seal is running.
        // Even a successful seal whose response was lost never sends commit.
        for status in [0, 200, 400, 409, 500, 503] {
            let mut row = UploadRow {
                status: SaveStatus::Saving,
                workspace: "retained-workspace".into(),
                expected_head: "base-head".into(),
                ..Default::default()
            };
            apply_upload_error(
                &mut row,
                false,
                &api::Error {
                    code: "SEAL_FAILED".into(),
                    message: "Seal response lost".into(),
                    retryable: true,
                    status,
                    request_id: None,
                    commit_attempted: None,
                },
            );
            assert_eq!(row.status, SaveStatus::Rejected);
            assert!(!row.status.pending());
            assert!(row.message.contains("Seal response lost"));
            assert_eq!(row.workspace, "retained-workspace");
            assert_eq!(row.expected_head, "base-head");
        }
    }

    #[test]
    fn only_attempted_commits_with_uncertain_results_block_the_upload_queue() {
        for status in [0, 200, 201, 500, 503] {
            assert_eq!(upload_error_status(true, status), SaveStatus::Unknown);
            assert!(upload_error_status(true, status).pending());
        }
        for status in [400, 401, 403, 409, 422] {
            assert_eq!(upload_error_status(true, status), SaveStatus::Rejected);
        }
    }

    #[test]
    fn starting_another_upload_retains_failed_staging_and_completed_receipts() {
        let mut uploads = vec![
            UploadRow {
                operation_id: "failed".into(),
                workspace: "retained-workspace".into(),
                message: "network interrupted".into(),
                status: SaveStatus::Rejected,
                ..Default::default()
            },
            UploadRow {
                status: SaveStatus::Published,
                ..Default::default()
            },
        ];
        append_uploads(&mut uploads, [UploadRow::default()]);
        assert_eq!(uploads.len(), 3);
        assert_eq!(uploads[0].workspace, "retained-workspace");
        assert_eq!(uploads[0].message, "network interrupted");
        assert_eq!(uploads[1].status, SaveStatus::Published);
        assert_eq!(uploads[2].status, SaveStatus::Waiting);
    }
    fn pending_model() -> Model {
        Model {
            directory: "reports".into(),
            selected: Some(
                json!({"kind":"file","name":"report.txt","path":"reports/report.txt","file_id":"file-a","selected_branch_id":"draft","version_id":"source","materialized":false,"inherited":true,"own_version_id":null,"source_version_id":"source"}),
            ),
            file_history: json!({"selected_branch_id":"draft","branches":[{"branch_id":"draft","current":true,"version_id":"source","materialized":false,"inherited":true,"own_version_id":null,"source_version_id":"source"}]}),
            ..Default::default()
        }
    }

    #[test]
    fn writing_an_existing_file_keeps_its_managed_name_path_and_pending_branch() {
        let mut model = pending_model();
        let target = upload_target(&model, "different-local-name.bin", true).unwrap();
        assert_eq!(
            target,
            UploadTarget {
                name: "report.txt".into(),
                path: "reports/report.txt".into(),
                branch_id: "draft".into()
            }
        );
        let queued = UploadRow {
            name: target.name,
            path: target.path,
            branch_id: target.branch_id,
            ..Default::default()
        };
        model.file_history["selected_branch_id"] = "later-selection".into();
        model.file_history["branches"] = json!([]);
        assert_eq!(
            write_session_body(&queued),
            json!({"path":"reports/report.txt","branch_id":"draft"}),
            "queued writes retain their original branch even after metadata changes"
        );
        model.selected = None;
        assert!(upload_target(&model, "different.txt", true).is_none());
    }
    #[test]
    fn regular_uploads_respect_existing_file_selections_without_reusing_another_files_branch() {
        let mut model = pending_model();
        model.entries = vec![
            json!({"kind":"file","name":"other.txt","path":"reports/other.txt","selected_branch_id":"other-draft"}),
        ];
        assert_eq!(
            upload_target(&model, "report.txt", false)
                .unwrap()
                .branch_id,
            "draft"
        );
        assert_eq!(
            upload_target(&model, "other.txt", false).unwrap().branch_id,
            "other-draft"
        );
        assert_eq!(
            upload_target(&model, "new.txt", false).unwrap().branch_id,
            ""
        );
        assert!(upload_target(&model, "../escape", false).is_none());
    }
    #[test]
    fn missing_visible_versions_never_start_preview_or_content_download_requests() {
        Owner::new().with(|| {
            let state = RwSignal::new(State {
                files: pending_model(),
                ..Default::default()
            });
            let mut entry = state.with_untracked(|s| s.files.selected.clone().unwrap());
            entry["version_id"] = Value::Null;
            preview(state, entry.clone());
            download(state, entry);
            assert_eq!(state.with_untracked(|s| s.files.preview_generation), 0);
            assert!(state.with_untracked(|s| s.files.preview.is_none()));
        });
    }

    #[test]
    fn directory_uploads_keep_independent_inherited_and_owned_file_branches() {
        let mut model = pending_model();
        model.selected = None;
        model.entries = vec![
            json!({"kind":"file","name":"report.txt","path":"reports/report.txt","selected_branch_id":"report-draft","version_id":"report-parent","inherited":true,"materialized":false,"own_version_id":null}),
            json!({"kind":"file","name":"other.txt","path":"reports/other.txt","selected_branch_id":"other-draft","version_id":"other-own","inherited":false,"materialized":true,"own_version_id":"other-own"}),
        ];
        assert!(model.entries.iter().all(visible_content));
        assert_eq!(
            upload_target(&model, "report.txt", false)
                .unwrap()
                .branch_id,
            "report-draft"
        );
        assert_eq!(
            upload_target(&model, "other.txt", false).unwrap().branch_id,
            "other-draft"
        );
    }

    #[test]
    fn an_unconfirmed_save_stops_only_files_that_have_not_been_uploaded() {
        let statuses = [
            SaveStatus::Published,
            SaveStatus::Unknown,
            SaveStatus::Waiting,
            SaveStatus::Degraded,
        ];
        let mut queue = statuses
            .into_iter()
            .map(|status| UploadRow {
                status,
                message: "existing receipt".into(),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        stop_waiting_uploads(&mut queue);
        assert_eq!(
            queue.iter().map(|row| row.status).collect::<Vec<_>>(),
            [
                SaveStatus::Published,
                SaveStatus::Unknown,
                SaveStatus::Rejected,
                SaveStatus::Degraded
            ]
        );
        assert!(queue[2].message.contains("not uploaded"));
        for index in [0, 1, 3] {
            assert_eq!(queue[index].message, "existing receipt");
        }
    }
}
