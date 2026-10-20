use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

pub const MAX_ERROR_CODE_BYTES: usize = 128;
pub const MAX_REQUEST_ID_BYTES: usize = 128;

/// A stable, machine-readable error identifier.
///
/// Codes are deliberately more restrictive than arbitrary strings so they can
/// be used safely in logs, metrics and client dispatch. Product-specific codes
/// may use `.` to form namespaces. ZXS's documented uppercase identifiers,
/// such as `DISK_UNAVAILABLE`, and lowercase transport identifiers are accepted;
/// mixed case is rejected to prevent ambiguous client dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ErrorCode(String);

impl ErrorCode {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidErrorCode> {
        let value = value.into();
        validate_error_code(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ErrorCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for ErrorCode {
    type Err = InvalidErrorCode;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for ErrorCode {
    type Error = InvalidErrorCode;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "error code must start with an ASCII letter, use one letter case, and contain at most 128 bytes of ASCII letters, digits, '.', '_' or '-'"
)]
pub struct InvalidErrorCode;

fn validate_error_code(value: &str) -> Result<(), InvalidErrorCode> {
    let mut bytes = value.bytes();
    let uppercase = value.as_bytes().first().is_some_and(u8::is_ascii_uppercase);
    if value.len() > MAX_ERROR_CODE_BYTES
        || !bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        || !bytes.all(|byte| {
            (if uppercase {
                byte.is_ascii_uppercase()
            } else {
                byte.is_ascii_lowercase()
            }) || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        return Err(InvalidErrorCode);
    }
    Ok(())
}

/// A bounded ASCII correlation identifier safe to copy into logs and headers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RequestId(String);

impl RequestId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidRequestId> {
        let value = value.into();
        validate_request_id(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for RequestId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for RequestId {
    type Err = InvalidRequestId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for RequestId {
    type Error = InvalidRequestId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("request ID must contain between 1 and 128 ASCII letters, digits, '.', '_', ':' or '-'")]
pub struct InvalidRequestId;

fn validate_request_id(value: &str) -> Result<(), InvalidRequestId> {
    if value.is_empty()
        || value.len() > MAX_REQUEST_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(InvalidRequestId);
    }
    Ok(())
}

/// Current JSON error contract shared by Rust services and Web clients.
///
/// `message` is display text and must never be used for client branching.
/// `details` is an object rather than arbitrary JSON so future fields remain
/// additive. Sensitive diagnostics belong in server logs, not this envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    pub code: ErrorCode,
    pub message: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present_request_id"
    )]
    pub request_id: Option<RequestId>,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub details: Map<String, Value>,
}

fn deserialize_present_request_id<'de, D>(deserializer: D) -> Result<Option<RequestId>, D::Error>
where
    D: Deserializer<'de>,
{
    RequestId::deserialize(deserializer).map(Some)
}

impl ErrorEnvelope {
    pub fn new(status: HttpStatus, message: impl Into<String>) -> Self {
        Self {
            code: status.error_code(),
            message: message.into(),
            request_id: None,
            retryable: status.default_retryable(),
            details: Map::new(),
        }
    }

    pub fn with_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
            retryable: false,
            details: Map::new(),
        }
    }

    pub fn with_request_id(
        mut self,
        request_id: impl Into<String>,
    ) -> Result<Self, InvalidRequestId> {
        self.request_id = Some(RequestId::new(request_id)?);
        Ok(self)
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HttpStatus {
    #[error("bad request")]
    BadRequest,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("unprocessable entity")]
    UnprocessableEntity,
    #[error("too many requests")]
    TooManyRequests,
    #[error("internal error")]
    Internal,
    #[error("service unavailable")]
    ServiceUnavailable,
}

impl HttpStatus {
    pub fn code(self) -> &'static str {
        match self {
            Self::BadRequest => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::UnprocessableEntity => "unprocessable_entity",
            Self::TooManyRequests => "too_many_requests",
            Self::Internal => "internal_error",
            Self::ServiceUnavailable => "service_unavailable",
        }
    }

    pub fn error_code(self) -> ErrorCode {
        ErrorCode::new(self.code()).expect("built-in HTTP error code is valid")
    }

    pub const fn status(self) -> u16 {
        match self {
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::UnprocessableEntity => 422,
            Self::TooManyRequests => 429,
            Self::Internal => 500,
            Self::ServiceUnavailable => 503,
        }
    }

    pub const fn default_retryable(self) -> bool {
        matches!(self, Self::TooManyRequests | Self::ServiceUnavailable)
    }
}

#[cfg(test)]
mod tests {
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
}
