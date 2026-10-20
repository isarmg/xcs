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
use serde_json::{Value, json};
use zxs_web::prelude::*;
use zxs_web::tasks::spawn_local;

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
pub(crate) fn check_upload(state: RwSignal<State>, id: &str) {
    let allowed = state.with_untracked(|s| {
        s.authenticated
            && !s.logout_pending
            && s.transfers
                .rows
                .iter()
                .any(|row| row.operation_id == id && row.queryable())
    });
    if !allowed {
        return;
    }
    let epoch = state.with_untracked(|s| s.session_epoch);
    let id = id.to_owned();
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
                update_upload(state, &id, |row| {
                    row.polling = false;
                    row.message = error.to_string();
                    row.request_id = error.request_id.clone();
                });
                if error.status == 401 {
                    fail(state, error);
                }
            }
        }
    });
}
pub(crate) fn poll(state: RwSignal<State>) {
    let snapshot = state.get_untracked();
    let epoch = snapshot.session_epoch;
    for row in snapshot.transfers.rows.iter().filter(|row| row.queryable()) {
        check_upload(state, &row.operation_id);
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
mod tests {
    use super::*;

    #[test]
    fn missing_or_unrelated_receipts_keep_the_original_mutation_pending() {
        Owner::new().with(|| {
            for response in [
                Value::Null,
                json!({}),
                json!({"state":"SUCCEEDED"}),
                json!({"operation_id":"other","state":"SUCCEEDED"}),
                json!({"operation":{"operation_id":"other","state":"SUCCEEDED"}}),
            ] {
                let state = RwSignal::new(State::default());
                assert_eq!(
                    record_receipt(state, "original", response),
                    SaveStatus::Unknown
                );
                state.with_untracked(|s| {
                    assert!(super::super::pending(s));
                    assert_eq!(s.files.operations.len(), 1);
                    assert_eq!(s.files.operations[0]["operation_id"], "original");
                });
                // A subsequent poll settles the same ID, rather than creating
                // another operation or leaving a permanent phantom lock.
                assert_eq!(
                    record_receipt(
                        state,
                        "original",
                        json!({
                            "operation_id":"original", "state":"SUCCEEDED"
                        })
                    ),
                    SaveStatus::Published
                );
                state.with_untracked(|s| {
                    assert!(!super::super::pending(s));
                    assert_eq!(s.files.operations.len(), 1);
                });
            }
        });
    }

    #[test]
    fn matching_wrapped_receipts_preserve_rejection_and_publication_evidence() {
        for status in ["REJECTED", "PUBLISHED", "PUBLISHED_DEGRADED", "IN_DOUBT"] {
            let value = json!({"operation":{"operation_id":"original","commit_state":status}});
            assert_eq!(receipt("original", value.clone()), value);
        }
    }
}
