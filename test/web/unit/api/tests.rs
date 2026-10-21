use super::*;
#[test]
fn verified_download_pipeline_rejects_body_completed_after_logout() {
    fn complete<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => panic!("test body should complete in one poll"),
        }
    }
    let bytes = b"verified real pipeline fixture".to_vec();
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let length = bytes.len() as u64;
    let admission = SessionAdmission::capture();
    assert_eq!(
        complete(verified_download_bytes(
            async { Ok(bytes.clone()) },
            length,
            &hash,
            admission,
        ))
        .unwrap(),
        bytes
    );
    let released = std::cell::Cell::new(false);
    let body = std::future::poll_fn(|_| {
        if released.get() {
            std::task::Poll::Ready(Ok(bytes.clone()))
        } else {
            std::task::Poll::Pending
        }
    });
    let mut pipeline = std::pin::pin!(verified_download_bytes(body, length, &hash, admission));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    use std::future::Future;
    assert!(pipeline.as_mut().poll(&mut context).is_pending());
    invalidate_session_work();
    released.set(true);
    let result = complete(pipeline);
    assert_eq!(result.unwrap_err().code, "SESSION_CHANGED");
}

#[test]
fn asynchronous_transfer_admission_rejects_a_replaced_session() {
    CREDENTIALS.with(|saved| {
        saved
            .borrow_mut()
            .replace(Some("old-token".into()), Some("old-csrf".into()));
    });
    let admission = SessionAdmission::capture();
    assert!(admission.check().is_ok());
    clear_session();
    assert_eq!(admission.check().unwrap_err().code, "SESSION_CHANGED");
    CREDENTIALS.with(|saved| {
        saved
            .borrow_mut()
            .replace(Some("new-token".into()), Some("new-csrf".into()));
    });
    assert_eq!(admission.check().unwrap_err().code, "SESSION_CHANGED");
    assert!(ensure_credential_generation(credential_generation()).is_ok());
    clear_session();
}

#[test]
fn beginning_logout_blocks_transfer_admission_before_revocation_completes() {
    CREDENTIALS.with(|saved| {
        saved
            .borrow_mut()
            .replace(Some("active-token".into()), Some("active-csrf".into()));
    });
    let mut state = crate::ui::State {
        authenticated: true,
        session_epoch: 7,
        ..Default::default()
    };
    // Multi-stage work has entered its internal await: outer UI checks
    // passed but upload, commit, download, and preview delivery must stop.
    let admissions = [SessionAdmission::capture(); 4];
    assert_eq!(crate::ui::begin_logout(&mut state), Some(8));
    assert!(state.logout_pending);
    assert!(state.authenticated);
    assert_eq!(crate::ui::begin_logout(&mut state), None);
    // The logout request has not completed. Each resumed transfer must stop.
    for admission in admissions {
        assert_eq!(admission.check().unwrap_err().code, "SESSION_CHANGED");
    }
    CREDENTIALS.with(|saved| {
        let saved = saved.borrow();
        assert_eq!(saved.token.as_deref(), Some("active-token"));
        assert_eq!(saved.csrf.as_deref(), Some("active-csrf"));
    });
    let logout_generation = credential_generation();
    assert!(ensure_credential_generation(logout_generation).is_ok());
    // A failed logout unlocks its retry, not the old transfer continuations.
    state.logout_pending = false;
    state.logout_unconfirmed = true;
    state.authenticated = false;
    assert_eq!(crate::ui::begin_logout(&mut state), Some(9));
    CREDENTIALS.with(|saved| assert_eq!(saved.borrow().token.as_deref(), Some("active-token")));
    assert!(admissions[0].check().is_err());
    clear_session();
}

#[test]
fn late_login_cannot_restore_cleared_or_replace_newer_credentials() {
    let mut saved = Credentials::default();
    let request_generation = saved.generation;
    saved.replace(None, None);
    assert!(!saved.replace_if_current(request_generation, Some("late".into()), None));
    assert!(saved.token.is_none());
    let current = saved.generation;
    assert!(saved.replace_if_current(current, Some("new".into()), Some("csrf".into())));
    assert!(!saved.replace_if_current(current, Some("late".into()), None));
    assert_eq!(saved.token.as_deref(), Some("new"));
    assert_eq!(saved.csrf.as_deref(), Some("csrf"));
}
#[test]
fn an_old_unauthorized_response_cannot_revoke_a_new_login() {
    let mut saved = Credentials::default();
    saved.replace(Some("old-token".into()), Some("old-csrf".into()));
    let old_request = saved.generation;
    saved.replace(Some("new-token".into()), Some("new-csrf".into()));
    assert!(!saved.revoke(old_request));
    assert_eq!(saved.token.as_deref(), Some("new-token"));
    assert_eq!(saved.csrf.as_deref(), Some("new-csrf"));
    assert!(saved.revoke(saved.generation));
    assert!(saved.token.is_none());
    assert!(saved.csrf.is_none());
    assert!(!saved.revoke(old_request));
}
#[test]
fn restrict_transport_to_own_api() {
    assert!(validate_path("/api/repositories").is_ok());
    assert!(validate_path("/api/file-manager/disks/P01/entries?path=report..txt").is_ok());
    assert!(validate_path("/api/file-manager/disks/P01/entries?path=photos%2F2026").is_ok());
    for path in [
        "https://other/api/a",
        "//other/api/a",
        "/other/api/a",
        "/v1/repositories",
        "/api/../secret",
        "/api/%2e%2e/secret",
        "/api/.%2e/secret",
        "/api/a%2fb",
        "/api/a#secret",
        "/api/\\evil",
        "/api/\t../secret",
        "/api/file\nname",
    ] {
        assert!(validate_path(path).is_err(), "{path}");
    }
}
