//! Publication receipts and polling of unconfirmed operations.
use super::{
    browse::load_entries,
    transfer::{apply_upload_result, update_upload},
};
use crate::{
    api,
    files::model::{SaveStatus, operation_status, publication_notice},
    ui::{State, current_session, fail, text},
};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use xcs_web::prelude::*;

pub(crate) fn record_operation(state: RwSignal<State>, value: Value) {
    let item = value.get("operation").unwrap_or(&value).clone();
    let id = text(&item, &["operation_id"]);
    let status = operation_status(&item);
    state.update(|s| {
        if !id.is_empty() {
            s.files
                .operations
                .retain(|operation| text(operation, &["operation_id"]) != id);
            s.files.operations.insert(0, item);
            s.files.operations.truncate(100);
        }
        crate::files::batch::settle_operation(&mut s.files, &id, status);
        crate::files::review::settle(&mut s.files, &id, status);
        s.notice = publication_notice(status);
    });
    if matches!(status, SaveStatus::Published | SaveStatus::Degraded) {
        load_entries(state);
    }
}
pub(crate) fn poll(state: RwSignal<State>) {
    let snapshot = state.get_untracked();
    let epoch = snapshot.session_epoch;
    for row in snapshot
        .files
        .uploads
        .iter()
        .filter(|row| row.status == SaveStatus::Unknown && !row.polling)
    {
        let id = row.operation_id.clone();
        update_upload(state, &id, |row| row.polling = true);
        spawn_local(async move {
            let result = api::request(
                "GET",
                &format!("/v1/operations/{}", api::segment(&id)),
                None,
            )
            .await;
            if !current_session(state, epoch) {
                return;
            }
            match result {
                Ok(value) => apply_upload_result(state, &id, value),
                Err(error) => {
                    update_upload(state, &id, |row| row.polling = false);
                    if error.status == 401 {
                        fail(state, error);
                    }
                }
            }
        });
    }
    for operation in snapshot
        .files
        .operations
        .iter()
        .filter(|operation| operation_status(operation) == SaveStatus::Unknown)
    {
        let id = text(operation, &["operation_id"]);
        if id.is_empty() || snapshot.files.polling_operations.contains(&id) {
            continue;
        }
        state.update(|s| s.files.polling_operations.push(id.clone()));
        spawn_local(async move {
            let result = api::request(
                "GET",
                &format!("/v1/operations/{}", api::segment(&id)),
                None,
            )
            .await;
            if !current_session(state, epoch) {
                return;
            }
            match result {
                Ok(value) => record_operation(state, value),
                Err(error) => {
                    if error.status == 401 {
                        fail(state, error)
                    }
                }
            }
            state.update(|s| s.files.polling_operations.retain(|item| item != &id));
        });
    }
}
