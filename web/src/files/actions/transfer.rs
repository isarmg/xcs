//! Sequential uploads, verified content reads and downloads.
use super::{browse::load_entries, disk_path, pending};
use crate::{
    api,
    files::model::{
        Model, SaveStatus, UploadRow, file_kind, join_path, operation_status, publication_notice,
        selected_branch, visible_content,
    },
    ui::{State, current_session, fail, set_notice, text},
};
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;
use xcs_web::{i18n::t, prelude::*};

pub(crate) fn upload_files(state: RwSignal<State>, files: Option<web_sys::FileList>) {
    queue_uploads(state, files, false);
}
/// A replacement uses the managed file's identity and current branch, regardless
/// of the local filename. It shares the ordinary upload/publication transaction.
pub(crate) fn upload_contents(state: RwSignal<State>, files: Option<web_sys::FileList>) {
    queue_uploads(state, files, true);
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
fn queue_uploads(state: RwSignal<State>, files: Option<web_sys::FileList>, replace_selected: bool) {
    let Some(files) = files else {
        return;
    };
    let snapshot = state.get_untracked();
    if !snapshot.files.writable || snapshot.files.loading || pending(&snapshot) {
        return;
    }
    if replace_selected && (files.length() != 1 || snapshot.files.history_loading) {
        return;
    }
    if files.length() > 128 {
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
    for index in 0..files.length() {
        if let Some(file) = files.get(index) {
            let Some(target) = upload_target(&snapshot.files, &file.name(), replace_selected)
            else {
                set_notice(
                    state,
                    t("上传文件名无效。", "An upload filename is invalid."),
                );
                return;
            };
            let id = api::operation_id();
            if id.is_empty() {
                return;
            }
            queue.push((
                file,
                UploadRow {
                    operation_id: id,
                    name: target.name,
                    slot: snapshot.files.slot.clone(),
                    path: target.path,
                    branch_id: target.branch_id,
                    ..Default::default()
                },
            ));
        }
    }
    state.update(|s| {
        append_uploads(
            &mut s.files.uploads,
            queue.iter().map(|(_, row)| row.clone()),
        );
    });
    let epoch = snapshot.session_epoch;
    spawn_local(async move {
        for (file, row) in queue {
            if !current_session(state, epoch) {
                return;
            }
            let id = row.operation_id.clone();
            update_upload(state, &id, |row| row.status = SaveStatus::Uploading);
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
                api::upload(&workspace, &api::entry_id(), &path, file).await?;
                if !current_session(state, epoch) {
                    return Err(api::session_changed());
                }
                update_upload(state, &id, |row| row.status = SaveStatus::Saving);
                api::commit_workspace(
                    &workspace,
                    &id,
                    &format!("Upload {}", row.name),
                    &expected_head,
                    || commit_attempted = true,
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
                    });
                    fail(state, error);
                }
            }
            if state.with_untracked(|s| {
                s.files
                    .uploads
                    .iter()
                    .find(|row| row.operation_id == id)
                    .is_some_and(|row| row.status == SaveStatus::Unknown)
            }) {
                state.update(|s| stop_waiting_uploads(&mut s.files.uploads));
                break;
            }
        }
    });
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
        row.message = t(
            "前一项保存尚未确认，本文件未上传。确认后请重新选择。",
            "The previous save is unconfirmed. This file was not uploaded; select it again after confirmation.",
        );
    }
}
pub(super) fn update_upload(state: RwSignal<State>, id: &str, update: impl FnOnce(&mut UploadRow)) {
    state.update(|s| {
        if let Some(row) = s
            .files
            .uploads
            .iter_mut()
            .find(|row| row.operation_id == id)
        {
            update(row);
        }
    });
}
pub(super) fn apply_upload_result(state: RwSignal<State>, id: &str, value: Value) {
    let status = operation_status(&value);
    update_upload(state, id, |row| {
        row.status = status;
        row.message = publication_notice(status);
        row.polling = false;
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
