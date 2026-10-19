//! Bounded JSON envelopes with HTTP status preserved for non-JSON failures.
use super::Error;
use futures_util::StreamExt;
use serde_json::Value;

pub(super) async fn decode(response: reqwest::Response) -> Result<Value, Error> {
    let status = response.status();
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
            return Err(Error::Response("JSON response exceeds client limit".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() && status.is_success() {
        return Ok(Value::Null);
    }
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) if !status.is_success() => {
            return Err(Error::Api {
                status: status.as_u16(),
                code: "HTTP_ERROR".into(),
                message: format!(
                    "server returned HTTP {} without a JSON error envelope",
                    status.as_u16()
                ),
                retryable: status.is_server_error() || status.as_u16() == 429,
            });
        }
        Err(_) => {
            return Err(Error::Response(format!(
                "server returned non-JSON HTTP {}",
                status.as_u16()
            )));
        }
    };
    if !status.is_success() {
        let error = &value["error"];
        let message = error["message"]
            .as_str()
            .unwrap_or("server rejected request");
        let message = match error["request_id"].as_str() {
            Some(request_id) => format!("{message} (request {request_id})"),
            None => message.to_owned(),
        };
        return Err(Error::Api {
            status: status.as_u16(),
            code: error["code"].as_str().unwrap_or("HTTP_ERROR").into(),
            message,
            retryable: error["retryable"].as_bool().unwrap_or(false),
        });
    }
    Ok(value)
}
