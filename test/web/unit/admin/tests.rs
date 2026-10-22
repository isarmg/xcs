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
