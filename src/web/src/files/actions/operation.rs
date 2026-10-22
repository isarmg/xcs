//! Publication receipts and polling of unconfirmed operations.
use super::browse::load_entries;
use crate::{
    api,
    files::model::{SaveStatus, operation_status, publication_notice},
    ui::{State, current_session, fail, text},
};
use json_codec::{Value, json};
use web_interface::prelude::*;
use web_interface::tasks::spawn_local;

/// A successful HTTP response is not a receipt unless it names our operation.
/// Keep the original ID pollable when a proxy returns empty/unrelated JSON.
pub(crate) fn record_receipt(state: RwSignal<State>, id: &str, value: Value) -> SaveStatus {
    let value = receipt(id, value);
    let status = operation_status(&value);
    record_operation(state, value);
    status
}
pub(super) fn receipt(id: &str, value: Value) -> Value {
    let item = value.get("operation").unwrap_or(&value);
    if item["operation_id"].as_str() == Some(id) {
        value
    } else {
        json!({"operation_id":id,"state":"in_doubt"})
    }
}

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
    if !snapshot.session_active() {
        return;
    }
    let epoch = snapshot.session_epoch;
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
                &format!("/api/operations/{}", api::segment(&id)),
                None,
            )
            .await;
            if !current_session(state, epoch) {
                return;
            }
            match result {
                Ok(value) => {
                    record_receipt(state, &id, value);
                }
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

#[cfg(test)]
#[path = "../../../../../test/web/unit/files/actions/operation/tests.rs"]
mod tests;
