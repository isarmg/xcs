//! Transport-independent client protocol. HTTP, credentials and I/O stay in adapters.
use serde_json::json;

/// Common JSON response cap, enforced while streaming and again when decoding.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseError {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub request_id: Option<String>,
}
impl ResponseError {
    pub fn new(status: u16, code: &str, message: &str) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            retryable: status >= 500 || status == 429,
            request_id: None,
        }
    }
}

/// Only an explicit client-error response proves that a submitted mutation was rejected.
pub fn mutation_outcome_unknown(status: u16) -> bool {
    !(400..500).contains(&status)
}

/// Missing IPC stage evidence must be treated conservatively.
pub fn publication_attempted(evidence: Option<bool>) -> bool {
    evidence != Some(false)
}

pub fn response_too_large(status: u16) -> ResponseError {
    ResponseError::new(
        status,
        "RESPONSE_TOO_LARGE",
        "JSON response exceeds client limit",
    )
}

use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;

struct ResponseJson(Value);

impl<'de> Deserialize<'de> for ResponseJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = ResponseJson;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON without duplicate fields or non-finite numbers")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<ResponseJson, E> {
                Ok(ResponseJson(Value::Bool(value)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<ResponseJson, E> {
                Ok(ResponseJson(Value::Null))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<ResponseJson, E> {
                Ok(ResponseJson(Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<ResponseJson, E> {
                Ok(ResponseJson(Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<ResponseJson, E> {
                Number::from_f64(value)
                    .map(|number| ResponseJson(Value::Number(number)))
                    .ok_or_else(|| E::custom("JSON numbers must be finite"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<ResponseJson, E> {
                Ok(ResponseJson(Value::String(value.into())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<ResponseJson, E> {
                Ok(ResponseJson(Value::String(value)))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<ResponseJson, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = access.next_element::<ResponseJson>()? {
                    values.push(value.0);
                }
                Ok(ResponseJson(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<ResponseJson, A::Error> {
                let mut values = Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate JSON field"));
                    }
                    values.insert(key, access.next_value::<ResponseJson>()?.0);
                }
                Ok(ResponseJson(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

fn strict_response_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = ResponseJson::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value.0)
}

/// Decode response-only JSON (telemetry may contain finite decimals). This does
/// not relax the integer-only parser used for persisted state and write bodies.
pub fn decode_response(status: u16, bytes: &[u8]) -> Result<Value, ResponseError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(response_too_large(status));
    }
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        strict_response_json(bytes).map_err(|_| {
            ResponseError::new(status, "INVALID_RESPONSE", "The response is not valid JSON")
        })?
    };
    if !(200..300).contains(&status) {
        let item = value.get("error").unwrap_or(&value);
        return Err(ResponseError {
            status,
            code: item["code"].as_str().unwrap_or("HTTP_ERROR").into(),
            message: item["message"]
                .as_str()
                .unwrap_or("The request failed")
                .into(),
            retryable: item["retryable"]
                .as_bool()
                .unwrap_or(status >= 500 || status == 429),
            request_id: item["request_id"].as_str().map(str::to_owned),
        });
    }
    Ok(value)
}

/// Bind a seal response to the requested workspace before adding caller-owned
/// fields and computing the canonical public commit envelope digest.
pub fn prepare_commit(
    mut seal: Value,
    workspace: &str,
    operation_id: &str,
    message: &str,
    expected_head: Option<&str>,
) -> crate::Result<Value> {
    if !seal.is_object() || seal["workspace_id"].as_str() != Some(workspace) {
        return Err(crate::Error::new(
            "INVALID_RESPONSE",
            "seal response has an invalid workspace identity",
        ));
    }
    let object = seal.as_object_mut().expect("validated object");
    // Replace the digest and caller metadata; retain the frozen workspace head
    // when the caller has no explicit precondition override.
    object.remove("request_digest");
    object.insert("operation_id".into(), operation_id.into());
    object.insert("message".into(), message.into());
    if let Some(head) = expected_head.filter(|head| !head.is_empty()) {
        object.insert("expected_head".into(), head.into());
    }
    let digest = crate::request_digest(
        "POST",
        "/v1/workspaces/{workspace}/commit",
        json!({"workspace":workspace}),
        json!({}),
        &seal,
    )?;
    seal["request_digest"] = digest.into();
    Ok(seal)
}

#[cfg(test)]
mod tests {
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
        let absent =
            prepare_commit(seal.clone(), "workspace_test", "op_test", "save", None).unwrap();
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
                "/v1/workspaces/{workspace}/commit",
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
}
