use serde::Deserialize;
use serde_json::json;

use super::*;

#[test]
fn error_codes_are_bounded_machine_identifiers() {
    for valid in [
        "DISK_UNAVAILABLE",
        "PROTECTION_INCOMPLETE",
        "bad_request",
        "sample.upload_conflict",
        "sample-product.rate-limited",
    ] {
        assert_eq!(valid.parse::<ErrorCode>().unwrap().as_str(), valid);
    }
    for invalid in ["", "BadRequest", "1bad", "has space", "échec"] {
        assert!(
            invalid.parse::<ErrorCode>().is_err(),
            "accepted {invalid:?}"
        );
    }
    assert!(ErrorCode::new("a".repeat(MAX_ERROR_CODE_BYTES)).is_ok());
    assert!(ErrorCode::new("a".repeat(MAX_ERROR_CODE_BYTES + 1)).is_err());
    assert!(serde_json::from_str::<ErrorCode>(r#""Bad Request""#).is_err());
}

#[test]
fn envelope_has_one_stable_wire_shape() {
    let envelope = ErrorEnvelope::new(HttpStatus::TooManyRequests, "try later")
        .with_request_id("request-1")
        .unwrap()
        .with_detail("retry_after", 5);
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        json!({
            "code": "too_many_requests",
            "message": "try later",
            "request_id": "request-1",
            "retryable": true,
            "details": { "retry_after": 5 }
        })
    );
    assert_eq!(
        serde_json::from_value::<ErrorEnvelope>(json!({
            "code": "not_found",
            "message": "missing",
            "retryable": false
        }))
        .unwrap(),
        ErrorEnvelope::new(HttpStatus::NotFound, "missing")
    );
}

#[test]
fn shared_error_fixtures_match_the_rust_wire_contract() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureSet {
        valid: Vec<Value>,
        invalid: Vec<Value>,
    }

    let fixtures: FixtureSet =
        serde_json::from_str(include_str!("fixtures/error-envelope.json")).unwrap();
    for (index, value) in fixtures.valid.into_iter().enumerate() {
        serde_json::from_value::<ErrorEnvelope>(value)
            .unwrap_or_else(|error| panic!("rejected valid shared fixture {index}: {error}"));
    }
    for (index, value) in fixtures.invalid.into_iter().enumerate() {
        assert!(
            serde_json::from_value::<ErrorEnvelope>(value).is_err(),
            "accepted invalid shared fixture {index}"
        );
    }
}

#[test]
fn request_ids_match_the_schema_identifier_contract() {
    for valid in [
        "request-1",
        "host:request_2",
        &"a".repeat(MAX_REQUEST_ID_BYTES),
    ] {
        assert_eq!(valid.parse::<RequestId>().unwrap().as_str(), valid);
    }
    for invalid in [
        "",
        "request id",
        "échec",
        &"a".repeat(MAX_REQUEST_ID_BYTES + 1),
    ] {
        assert!(
            invalid.parse::<RequestId>().is_err(),
            "accepted {invalid:?}"
        );
    }
}

#[test]
fn status_mapping_and_retry_defaults_are_explicit() {
    let cases = [
        (HttpStatus::BadRequest, 400, false),
        (HttpStatus::Unauthorized, 401, false),
        (HttpStatus::Forbidden, 403, false),
        (HttpStatus::NotFound, 404, false),
        (HttpStatus::Conflict, 409, false),
        (HttpStatus::UnprocessableEntity, 422, false),
        (HttpStatus::TooManyRequests, 429, true),
        (HttpStatus::Internal, 500, false),
        (HttpStatus::ServiceUnavailable, 503, true),
    ];
    for (status, expected_code, retryable) in cases {
        assert_eq!(status.status(), expected_code);
        assert_eq!(status.default_retryable(), retryable);
        assert!(ErrorCode::new(status.code()).is_ok());
    }
}
