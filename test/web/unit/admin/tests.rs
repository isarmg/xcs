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

#[test]
fn saving_an_account_invalidates_old_rows_before_refresh_and_never_regrants_from_them() {
    let mut model = Model::default();
    let old = json_codec::json!({"accounts":[{"username":"reader","admin":true,"disabled":false,"repositories":["private"]}]});
    accept(&mut model, "GET", "/api/admin/accounts", false, old);
    assert_eq!(account_fields(&model, "reader").unwrap()["admin"], "true");
    accept(
        &mut model,
        "POST",
        "/api/admin/accounts",
        false,
        json_codec::json!({"updated":true}),
    );
    assert!(account_write("POST", "/api/admin/accounts"));
    assert!(model.accounts.is_empty());
    assert!(
        account_fields(&model, "reader").is_none(),
        "a queued old row click cannot repopulate revoked grants"
    );
    // Failed follow-up reads do not restore the invalidated row.
    reject(
        &mut model,
        "GET",
        false,
        "",
        &api::Error {
            status: 503,
            code: "UNAVAILABLE".into(),
            message: "refresh failed".into(),
            retryable: true,
            request_id: None,
        },
    );
    assert!(account_fields(&model, "reader").is_none());
    let refreshed = json_codec::json!({"accounts":[{"username":"reader","admin":false,"disabled":true,"repositories":[]}]});
    accept(&mut model, "GET", "/api/admin/accounts", false, refreshed);
    let mut fields = account_fields(&model, "reader").unwrap();
    fields.insert("password".into(), "new password".into());
    let (_, _, body) = request::build("account", &fields, "").unwrap();
    assert_eq!(body["admin"], false);
    assert_eq!(body["disabled"], true);
    assert_eq!(body["repositories"], json_codec::json!([]));
}

#[test]
fn account_row_handlers_do_not_bypass_busy_uncertain_or_session_guards() {
    Owner::new().with(|| {
        let state = RwSignal::new(State { authenticated:true, admin:true, ..Default::default() });
        state.update(|s| {
            s.admin_console.accounts = vec![json_codec::json!({"username":"reader","admin":true,"disabled":false,"repositories":[]})];
            s.admin_console.fields.insert("username".into(), "unchanged".into());
        });
        for guard in ["busy", "unknown", "logout"] {
            state.update(|s| {
                s.admin_console.busy = guard == "busy";
                s.admin_console.uncertain_operation = if guard == "unknown" {"unknown".into()} else {String::new()};
                s.logout_pending = guard == "logout";
            });
            select_account(state, "reader");
            assert_eq!(state.with_untracked(|s| s.admin_console.fields["username"].clone()), "unchanged");
        }
    });
}
