use super::*;

#[test]
fn write_admission_combines_session_location_and_publication_state() {
    let mut state = State {
        authenticated: true,
        ..Default::default()
    };
    state.files.writable = true;
    assert!(!write_allowed(&state), "unknown health fails closed");
    state.management.storage = crate::management::Storage::Ready;
    assert!(write_allowed(&state));
    state.restoring = true;
    assert!(!write_allowed(&state));
    state.restoring = false;
    state.files.loading = true;
    assert!(!write_allowed(&state));
    state.files.loading = false;
    state.files.mutation_pending = true;
    assert!(!write_allowed(&state));
    state.files.mutation_pending = false;
    assert!(write_allowed(&state));
    crate::ui::begin_logout(&mut state);
    assert!(state.authenticated, "revocation has not been confirmed");
    assert!(!write_allowed(&state));
    state.logout_pending = false;
    state.authenticated = false;
    state.logout_unconfirmed = true;
    assert!(
        !write_allowed(&state),
        "failed logout permits revocation retry only"
    );
}
