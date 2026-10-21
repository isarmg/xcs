use super::super::pending;
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
    assert!(invalid.path.is_empty());
    assert!(invalid.branch_id.is_empty());
}
#[test]
fn missing_strong_id_cannot_create_a_preflight_record_with_an_empty_key() {
    assert!(
        preflight_failure(
            &Model::default(),
            "large.bin".into(),
            None,
            Some(67_108_865),
            String::new(),
            "Too large".into(),
        )
        .is_none()
    );
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
            selected: Some(json!({"name":"file","path":"folder/file","selected_branch_id":"main"})),
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
            authenticated: true,
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
fn sign_out_pending_rejects_new_content_delivery_before_scheduling() {
    Owner::new().with(|| {
        let state = RwSignal::new(State {
            authenticated: true,
            logout_pending: true,
            files: pending_model(),
            ..Default::default()
        });
        let entry = json!({"name":"private.txt","version_id":"private-version"});
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
