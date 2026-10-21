use super::*;

#[test]
fn seal_identity_is_checked_before_preparation() {
    for seal in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"workspace_id":"other"}),
    ] {
        assert_eq!(
            prepare_commit(seal, "workspace_test", "op_test", "save", None)
                .unwrap_err()
                .code,
            "INVALID_RESPONSE"
        );
    }
}

#[test]
fn preparation_has_one_canonical_precondition_and_digest() {
    let seal = json!({"workspace_id":"workspace_test", "manifest_digest":"abc", "expected_head":"sealed_head", "request_digest":"stale", "operation_id":"stale", "message":"stale"});
    let absent = prepare_commit(seal.clone(), "workspace_test", "op_test", "save", None).unwrap();
    let empty =
        prepare_commit(seal.clone(), "workspace_test", "op_test", "save", Some("")).unwrap();
    assert_eq!(absent, empty);
    assert_eq!(absent["expected_head"], "sealed_head");
    assert_eq!(absent["operation_id"], "op_test");
    assert_eq!(absent["message"], "save");
    let with_head =
        prepare_commit(seal, "workspace_test", "op_test", "save", Some("head_test")).unwrap();
    assert_eq!(with_head["expected_head"], "head_test");
    assert_ne!(with_head["request_digest"], absent["request_digest"]);
    assert_eq!(
        absent["request_digest"],
        crate::request_digest(
            "POST",
            "/api/workspaces/{workspace}/commit",
            json!({"workspace":"workspace_test"}),
            json!({}),
            &absent
        )
        .unwrap()
    );
}

#[test]
fn response_json_rejects_duplicate_aliases_and_retains_telemetry_decimals() {
    for body in [
        br#"{"token":"a","token":"b"}"#.as_slice(),
        br#"{"items":[{"code":1,"co\u0064e":2}]}"#,
        b"{} {}",
        br#"{"x":1e999}"#,
    ] {
        assert_eq!(
            decode_response(200, body).unwrap_err().code,
            "INVALID_RESPONSE"
        );
    }
    let body = br#"{"cpu":8.899999618530273,"bytes":"18446744073709551615"}"#;
    assert!(decode_response(200, body).unwrap()["cpu"].is_f64());
    assert!(crate::strict_json(body).is_err());
}

#[test]
fn status_and_publication_classification_survive_all_response_failures() {
    for status in [0, 200, 204, 302, 400, 401, 409, 422, 429, 500, 503] {
        let failure = decode_response(status, b"secret-not-json").unwrap_err();
        assert_eq!(failure.status, status);
        assert!(!failure.message.contains("secret"));
        assert_eq!(
            mutation_outcome_unknown(status),
            !(400..500).contains(&status)
        );
    }
    assert!(!publication_attempted(Some(false)));
    assert!(publication_attempted(Some(true)));
    assert!(publication_attempted(None));
    assert_eq!(decode_response(204, b"").unwrap(), Value::Null);
    assert_eq!(decode_response(404, b"").unwrap_err().code, "HTTP_ERROR");
}

#[test]
fn response_envelopes_preserve_fields_and_share_a_bounded_size() {
    let error = decode_response(503, br#"{"error":{"code":"OFFLINE","message":"offline","retryable":false,"request_id":"req_test"}}"#).unwrap_err();
    assert_eq!(error.code, "OFFLINE");
    assert_eq!(error.request_id.as_deref(), Some("req_test"));
    assert!(!error.retryable);
    let mut bytes = vec![b' '; MAX_RESPONSE_BYTES];
    bytes[..2].copy_from_slice(b"{}");
    assert!(decode_response(200, &bytes).is_ok());
    bytes.push(b' ');
    let error = decode_response(409, &bytes).unwrap_err();
    assert_eq!(error.code, "RESPONSE_TOO_LARGE");
    assert_eq!(error.status, 409);
}
