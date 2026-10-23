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
    state.session_transition = crate::ui::SessionTransition::LogoutUnconfirmed;
    state.authenticated = false;
    assert_eq!(crate::ui::begin_logout(&mut state), Some(2));
    CREDENTIALS.with(|saved| {
        let saved = saved.borrow();
        assert_eq!(saved.token.as_deref(), Some("active-token"));
        assert_eq!(saved.csrf.as_deref(), Some("active-csrf"));
    });
    clear_session();
}

fn session_response() -> Value {
    json!({"token":"new-token","csrf_token":"new-csrf","principal":"reader","admin":false})
}

#[test]
fn malformed_authentication_success_cannot_install_or_restore_a_session() {
    let original = Credentials {
        token: Some("existing-token".into()),
        csrf: Some("existing-csrf".into()),
        generation: 10,
    };
    let mut invalid = vec![Value::Null, json!({}), json!({"token":"new-token"})];
    for field in ["principal", "csrf_token", "admin"] {
        for replacement in [Value::Null, json!(""), json!(42)] {
            let mut response = session_response();
            response[field] = replacement;
            invalid.push(response);
        }
    }
    for value in invalid {
        for login in [false, true] {
            CREDENTIALS.with(|saved| *saved.borrow_mut() = original.clone());
            let result = if login {
                accept_login(original.generation, value.clone())
            } else {
                accept_restore(original.generation, value.clone())
            };
            let error =
                result.expect_err("incomplete identities must not open the authenticated UI");
            assert_eq!(error.code, "INVALID_SESSION");
            assert_eq!(error.status, 200);
            CREDENTIALS.with(|saved| {
                let saved = saved.borrow();
                assert_eq!(saved.generation, original.generation);
                assert_eq!(saved.token, original.token);
                assert_eq!(saved.csrf, original.csrf);
            });
        }
    }
    for token in [Value::Null, json!(""), json!(42)] {
        let mut value = session_response();
        value["token"] = token;
        assert_eq!(
            accept_login(original.generation, value).unwrap_err().code,
            "INVALID_SESSION"
        );
    }
    clear_session();
}

#[test]
fn validated_session_restoration_requires_the_original_credential_generation() {
    CREDENTIALS.with(|saved| *saved.borrow_mut() = Credentials::default());
    let mut response = session_response();
    response.as_object_mut().unwrap().remove("token");
    assert!(accept_restore(0, response.clone()).is_ok());
    CREDENTIALS.with(|saved| {
        let saved = saved.borrow();
        assert!(
            saved.token.is_none(),
            "restoration uses the HTTP-only cookie"
        );
        assert_eq!(saved.csrf.as_deref(), Some("new-csrf"));
    });
    assert!(accept_login(0, session_response()).is_ok());
    assert_eq!(
        accept_restore(0, response).unwrap_err().code,
        "SESSION_CHANGED"
    );
    CREDENTIALS.with(|saved| {
        let saved = saved.borrow();
        assert_eq!(saved.generation, 1);
        assert_eq!(saved.token.as_deref(), Some("new-token"));
        assert_eq!(saved.csrf.as_deref(), Some("new-csrf"));
    });
    clear_session();
}
