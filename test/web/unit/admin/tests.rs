use super::*;
use crate::management::Storage;
#[test]
fn inspection_survives_data_failure_but_storage_writes_require_an_active_session() {
    let mut state = State {
        authenticated: true,
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
    state.session_transition = crate::ui::SessionTransition::LoggingOut;
    assert!(!permitted(&state, false));
    state.session_transition = crate::ui::SessionTransition::Idle;
    state.authenticated = false;
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
fn inspection_snapshots_never_settle_an_interrupted_storage_mutation() {
    let mut model = Model {
        uncertain_operation: "original".into(),
        ..Default::default()
    };
    for value in [
        json_codec::json!([]),
        json_codec::json!([{"repository_id":"repo"}]),
    ] {
        accept(&mut model, "GET", "/api/repositories", false, value);
        assert_eq!(model.uncertain_operation, "original");
        assert!(model.reload_blocked());
    }
}

#[test]
fn management_selection_does_not_bypass_session_busy_or_uncertainty_guards() {
    Owner::new().with(|| {
        let state = RwSignal::new(State {
            authenticated: true,
            ..Default::default()
        });
        for guard in [
            "anonymous",
            "restoring",
            "logout",
            "unconfirmed",
            "busy",
            "unknown",
        ] {
            state.update(|s| {
                s.authenticated = guard != "anonymous";
                s.admin_console.busy = guard == "busy";
                s.admin_console.uncertain_operation = if guard == "unknown" {
                    "original".into()
                } else {
                    String::new()
                };
                s.session_transition = match guard {
                    "restoring" => crate::ui::SessionTransition::Restoring,
                    "logout" => crate::ui::SessionTransition::LoggingOut,
                    "unconfirmed" => crate::ui::SessionTransition::LogoutUnconfirmed,
                    _ => crate::ui::SessionTransition::Idle,
                };
            });
            select(state, "repository");
            assert_eq!(
                state.with_untracked(|s| s.admin_console.kind.clone()),
                "inspect"
            );
        }
        state.update(|s| {
            *s = State {
                authenticated: true,
                ..Default::default()
            };
        });
        select(state, "repository");
        assert_eq!(
            state.with_untracked(|s| s.admin_console.kind.clone()),
            "repository"
        );
    });
}
