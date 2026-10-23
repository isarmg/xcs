use super::*;
#[test]
fn segment_encoding_is_native_safe_and_matches_uri_component_semantics() {
    assert_eq!(segment("AZaz09-_.!~*'()"), "AZaz09-_.!~*'()");
    assert_eq!(
        segment("a/b c?d=e&f#%+\\\u{0}"),
        "a%2Fb%20c%3Fd%3De%26f%23%25%2B%5C%00"
    );
    assert_eq!(segment("历史📁"), "%E5%8E%86%E5%8F%B2%F0%9F%93%81");
    assert_eq!(segment(""), "");
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

#[test]
fn beginning_logout_invalidates_pending_login_without_losing_revocation_credentials() {
    CREDENTIALS.with(|saved| {
        saved
            .borrow_mut()
            .replace(Some("active-token".into()), Some("active-csrf".into()));
    });
    let generation = CREDENTIALS.with(|saved| saved.borrow().generation);
    let mut state = crate::ui::State {
        authenticated: true,
        ..Default::default()
    };
    assert_eq!(crate::ui::begin_logout(&mut state), Some(1));
    assert_eq!(crate::ui::begin_logout(&mut state), None);
    CREDENTIALS.with(|saved| {
        let mut saved = saved.borrow_mut();
        assert!(!saved.replace_if_current(generation, Some("late-token".into()), None));
        assert_eq!(saved.token.as_deref(), Some("active-token"));
        assert_eq!(saved.csrf.as_deref(), Some("active-csrf"));
    });
    // Transport failure locks the UI but must retain the exact credential for
    // a later revocation retry, rather than replacing it with a new identity.
    state.logout_pending = false;
    state.logout_unconfirmed = true;
    state.authenticated = false;
    assert_eq!(crate::ui::begin_logout(&mut state), Some(2));
    CREDENTIALS.with(|saved| {
        let saved = saved.borrow();
        assert_eq!(saved.token.as_deref(), Some("active-token"));
        assert_eq!(saved.csrf.as_deref(), Some("active-csrf"));
    });
    clear_session();
}
