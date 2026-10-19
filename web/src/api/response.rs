//! HTTP responses include transient telemetry decimals. Persisted state and
//! write requests retain the contract's separate integer-only JSON parser.
use super::{Error, error};
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;
use xcs_web::i18n::t;

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

pub(super) fn decode(status: u16, bytes: &[u8]) -> Result<Value, Error> {
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        strict_response_json(bytes).map_err(|_| {
            // Never include response bytes: a malformed login/error response
            // can contain secrets or a proxy's diagnostic HTML.
            error(
                "INVALID_RESPONSE",
                t("响应不是有效 JSON", "The response is not valid JSON"),
            )
            .with_status(status)
        })?
    };
    if !(200..300).contains(&status) {
        let item = value.get("error").unwrap_or(&value);
        return Err(Error {
            code: item["code"].as_str().unwrap_or("HTTP_ERROR").into(),
            message: item["message"]
                .as_str()
                .unwrap_or("The request failed")
                .into(),
            retryable: item["retryable"].as_bool().unwrap_or(false),
            status,
            request_id: item["request_id"].as_str().map(str::to_owned),
            commit_attempted: None,
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_cpu_response_decimals_and_full_width_byte_strings_are_accepted() {
        let bytes = br#"{"cpu":{"usage_percent":8.899999618530273,"logical_count":8},"memory":{"total_bytes":"18446744073709551615"},"disks":[]}"#;
        let value = decode(200, bytes).unwrap();
        assert_eq!(value["cpu"]["usage_percent"], 8.899999618530273);
        assert_eq!(value["cpu"]["logical_count"].as_u64(), Some(8));
        assert_eq!(value["memory"]["total_bytes"], u64::MAX.to_string());
        // This response-only exception must not weaken persisted/write JSON.
        assert!(xcs_contract::strict_json(bytes).is_err());
    }

    #[test]
    fn first_cpu_sample_can_remain_unavailable() {
        let value = decode(200, br#"{"cpu":{"usage_percent":null}}"#).unwrap();
        assert!(value["cpu"]["usage_percent"].is_null());
    }

    #[test]
    fn duplicate_keys_are_rejected_at_every_depth_including_escaped_aliases() {
        for bytes in [
            br#"{"token":"first","token":"second"}"#.as_slice(),
            br#"{"cpu":{"usage_percent":1.2,"usage_percent":2.3}}"#,
            br#"{"items":[{"code":"first","co\u0064e":"second"}]}"#,
        ] {
            let error = decode(200, bytes).unwrap_err();
            assert_eq!(error.code, "INVALID_RESPONSE");
            assert_eq!(error.status, 200);
        }
    }

    #[test]
    fn malformed_json_and_non_finite_numbers_are_rejected_without_exposing_body() {
        for bytes in [
            b"\xef\xbb\xbf{}".as_slice(),
            br#"{"cpu":NaN}"#,
            br#"{"cpu":Infinity}"#,
            br#"{"cpu":1e999}"#,
            br#"{"cpu":"\ud800"}"#,
            b"{} {}",
            b"<html>private-token</html>",
        ] {
            let error = decode(502, bytes).unwrap_err();
            assert_eq!(error.code, "INVALID_RESPONSE");
            assert_eq!(error.status, 502);
            assert!(!error.message.contains("private-token"));
            assert!(!error.message.contains("HTML"));
        }
    }

    #[test]
    fn json_errors_keep_actual_status_code_retryability_and_request_id() {
        let bytes = br#"{"error":{"code":"DISK_OFFLINE","message":"A disk is offline","retryable":true,"request_id":"req_test"}}"#;
        let error = decode(503, bytes).unwrap_err();
        assert_eq!(error.status, 503);
        assert_eq!(error.code, "DISK_OFFLINE");
        assert_eq!(error.message, "A disk is offline");
        assert!(error.retryable);
        assert_eq!(error.request_id.as_deref(), Some("req_test"));
    }

    #[test]
    fn empty_http_errors_keep_their_status() {
        let error = decode(404, b"").unwrap_err();
        assert_eq!(error.status, 404);
        assert_eq!(error.code, "HTTP_ERROR");
    }
}
