use super::*;
use crate::management::Storage;
#[test]
fn account_administration_survives_data_failure_but_storage_writes_fail_closed() {
    let mut state = State {
        authenticated: true,
        admin: true,
        ..Default::default()
    };
    for storage in [
        Storage::Unknown,
        Storage::ReadOnly,
        Storage::Unavailable,
        Storage::Unconfigured,
        Storage::RecoveryRequired,
    ] {
        state.management.storage = storage;
        assert!(permitted(&state, false));
        assert!(!permitted(&state, true));
    }
    state.management.storage = Storage::Ready;
    assert!(permitted(&state, true));
    state.admin_console.busy = true;
    assert!(!permitted(&state, false));
    state.admin_console.busy = false;
    state.logout_pending = true;
    assert!(!permitted(&state, false));
    state.logout_pending = false;
    state.admin = false;
    assert!(!permitted(&state, false));
}

#[test]
fn language_reload_preserves_admin_operation_fences() {
    let mut model = Model::default();
    assert!(!model.reload_blocked());
    model.busy = true;
    assert!(model.reload_blocked());
    model.busy = false;
    model.uncertain_operation = "original".into();
    assert!(model.reload_blocked());
    model.uncertain_operation = "unknown".into();
    assert!(model.reload_blocked());
}

#[test]
fn invalid_success_and_early_not_found_preserve_review_and_uncertainty() {
    let reviewed = json_codec::json!({"plan_id":"p", "digest":"reviewed", "state":"ready"});
    for response in [
        Value::Null,
        json_codec::json!({}),
        json_codec::json!({"operation_id":"wrong","commit_state":"PUBLISHED"}),
    ] {
        let mut model = Model {
            plan: reviewed.clone(),
            acknowledged: true,
            uncertain_operation: "original".into(),
            ..Default::default()
        };
        let contract =
            response::Contract::new("POST", "/api/plans/p/commit", None, "original", "admin");
        match contract.validate(response) {
            Ok(value) => accept(&mut model, "POST", "/api/plans/p/commit", false, value),
            Err(error) => reject(&mut model, "POST", false, "original", &error),
        }
        assert_eq!(model.plan, reviewed);
        assert!(model.acknowledged);
        assert_eq!(model.uncertain_operation, "original");
    }
    // The operation may not be indexed yet. A lookup error is not evidence of
    // rejection and never enters the success reducer.
    let error = api::Error {
        code: "NOT_FOUND".into(),
        message: "not indexed".into(),
        status: 404,
        retryable: false,
        request_id: None,
    };
    let mut model = Model {
        plan: reviewed.clone(),
        uncertain_operation: "original".into(),
        ..Default::default()
    };
    reject(&mut model, "GET", false, "", &error);
    assert_eq!(model.plan, reviewed);
    assert!(model.reload_blocked());
}

#[test]
fn account_snapshots_never_settle_an_interrupted_mutation() {
    let mut model = Model::default();
    reject(
        &mut model,
        "POST",
        false,
        "",
        &api::Error {
            code: "TRANSPORT".into(),
            message: "response lost".into(),
            status: 0,
            retryable: false,
            request_id: None,
        },
    );
    assert_eq!(model.uncertain_operation, "unknown");
    // Both an older snapshot and one showing the requested value are insufficient:
    // neither identifies the original request or proves its password outcome.
    for disabled in [false, true, false] {
        let value = json_codec::json!({"accounts":[{"username":"reader", "admin":false, "disabled":disabled, "repositories":[]}]});
        let value = response::Contract::new("GET", "/api/admin/accounts", None, "", "admin")
            .validate(value)
            .unwrap();
        accept(&mut model, "GET", "/api/admin/accounts", false, value);
        assert_eq!(model.accounts[0]["disabled"], disabled);
        assert_eq!(model.uncertain_operation, "unknown");
        assert!(model.reload_blocked());
    }
}
