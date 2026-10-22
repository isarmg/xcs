use super::*;

#[test]
fn only_explicit_ready_status_admits_writes() {
    for (wire, expected) in [
        ("ready", Storage::Ready),
        ("read_only", Storage::ReadOnly),
        ("unavailable", Storage::Unavailable),
        ("unconfigured", Storage::Unconfigured),
        ("recovery_required", Storage::RecoveryRequired),
        ("future_state", Storage::Unknown),
    ] {
        let state = Storage::parse(&json!({"state":wire}));
        assert_eq!(state, expected);
        assert_eq!(state.writable(), wire == "ready");
    }
    assert!(!Storage::parse(&Value::Null).writable());
    assert!(!Storage::default().writable());
}

#[test]
fn missing_authentication_never_becomes_new_installation() {
    let mut model = Model::default();
    accept(
        &mut model,
        &json!({"authentication":"recovery_required", "storage":{"state":"unavailable", "retryable":true}}),
    );
    assert!(model.auth_recovery);
    assert_eq!(model.storage, Storage::Unavailable);
    assert_ne!(model.storage, Storage::Unconfigured);
    assert!(model.retryable);
    assert!(!model.storage.writable());
}

#[test]
fn health_failure_does_not_revoke_session_or_overwrite_data_outcomes() {
    let mut state = State {
        authenticated: true,
        principal: "admin".into(),
        admin: true,
        ..Default::default()
    };
    state.management.storage = Storage::RecoveryRequired;
    assert!(state.session_active());
    assert_eq!(state.principal, "admin");
    state.management.storage = Storage::ReadOnly;
    assert!(state.session_active());
    assert!(state.management.storage.readable());
    assert!(!state.management.storage.writable());
}
