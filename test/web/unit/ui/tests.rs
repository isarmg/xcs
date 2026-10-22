use super::*;
#[test]
fn session_transitions_reject_duplicate_and_overlapping_submissions() {
    let mut state = State::default();
    assert_eq!(begin_restore(&mut state), Some(1));
    assert_eq!(begin_restore(&mut state), None);
    assert_eq!(begin_login(&mut state), None);
    assert_eq!(state.session_epoch, 1);
    state.restoring = false;
    assert_eq!(begin_login(&mut state), Some(2));
    assert_eq!(begin_login(&mut state), None);
    assert_eq!(begin_restore(&mut state), None);
    assert_eq!(state.busy, 1);
    state.busy = 0;
    state.logout_pending = true;
    assert_eq!(begin_login(&mut state), None);
    assert_eq!(begin_restore(&mut state), None);
    assert_eq!(state.session_epoch, 2);
}
#[test]
fn content_and_account_completion_require_the_original_active_session() {
    Owner::new().with(|| {
        let state = RwSignal::new(State {
            authenticated: true,
            session_epoch: 4,
            ..Default::default()
        });
        assert!(current_session(state, 4));
        state.update(|s| s.logout_pending = true);
        assert!(!current_session(state, 4));
        state.update(|s| {
            s.logout_pending = false;
            s.session_epoch = 5;
        });
        assert!(!current_session(state, 4));
        assert!(current_session(state, 5));
        end_session(state, "Password changed".into());
        assert!(!current_session(state, 5));
        state.with_untracked(|s| {
            assert_eq!(s.session_epoch, 6);
            assert_eq!(s.notice, "Password changed");
            assert!(!s.authenticated);
        });
    });
}
#[test]
fn detached_completion_rejects_a_disposed_root() {
    let owner = Owner::new();
    let state = owner.with(|| {
        RwSignal::new(State {
            authenticated: true,
            session_epoch: 4,
            ..Default::default()
        })
    });
    assert!(current_session(state, 4));
    owner.cleanup();
    assert!(!current_session(state, 4));
    assert_eq!(state.try_with_untracked(|s| s.logout_pending), None);
}
#[test]
fn shell_identity_does_not_change_with_file_progress() {
    let mut state = State {
        authenticated: true,
        principal: "writer".into(),
        ..Default::default()
    };
    let session = session_view(&state);
    state.busy = 3;
    state.password_pending = true;
    state.notice = "saving".into();
    assert_eq!(session_view(&state), session);
    assert_eq!(session.phase, SessionPhase::Authenticated);
}
#[test]
fn wrong_password_does_not_discard_pending_file_saves() {
    let mut state = State {
        authenticated: true,
        password_pending: true,
        ..Default::default()
    };
    state.files.slot = "P01".into();
    reject_password_change(
        &mut state,
        &api::Error {
            code: "CURRENT_PASSWORD_INVALID".into(),
            message: "Incorrect password".into(),
            status: 403,
            retryable: false,
            request_id: Some("req_test".into()),
        },
    );
    assert!(state.authenticated);
    assert_eq!(state.files.slot, "P01");
    assert!(!state.password_pending);
    assert_eq!(
        state.password_failure.unwrap().request_id.as_deref(),
        Some("req_test")
    );
}

#[test]
fn failed_session_restore_keeps_login_available_during_storage_recovery() {
    let mut state = State {
        session_failure: Some("Authentication service unavailable".into()),
        ..Default::default()
    };
    state.management.storage = crate::management::Storage::RecoveryRequired;
    assert_eq!(session_view(&state).phase, SessionPhase::Anonymous);
    assert_eq!(begin_login(&mut state), Some(1));
    assert!(!state.management.storage.writable());
}

#[test]
fn logout_clears_management_and_administration_state() {
    Owner::new().with(|| {
        let state = RwSignal::new(State {
            authenticated: true,
            session_epoch: 9,
            ..Default::default()
        });
        state.update(|s| s.management.storage = crate::management::Storage::Ready);
        end_session(state, String::new());
        state.with_untracked(|s| {
            assert_eq!(s.session_epoch, 10);
            assert!(!s.authenticated);
            assert_eq!(s.management.storage, crate::management::Storage::Unknown);
        });
    });
}
