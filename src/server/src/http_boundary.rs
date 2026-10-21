//! Shared HTTP rejections and bounded request decoding.
//!
//! Translate failures before a response starts. Content stream errors remain
//! transport failures; they cannot be replaced by a JSON envelope after headers.
use data_serialization::{Deserialize, Serialize, de::DeserializeOwned};
use http_routing::{
    Json,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, FromRequestParts, Path, Query, Request},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use json_codec::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub request_id: String,
    #[serialization(skip)]
    pub status: u16,
}
impl ApiError {
    pub fn new(status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            retryable: (status >= 500 && status != 501) || status == 429,
            request_id: storage_contract::new_id("req"),
        }
    }
    pub fn invalid(error: impl std::fmt::Display) -> Self {
        Self::new(400, "INVALID_REQUEST", error.to_string())
    }
    pub(crate) fn join(error: tokio::task::JoinError) -> Self {
        tracing::error!(%error, "blocking request task failed");
        Self::new(500, "INTERNAL_ERROR", "request task failed")
    }
    pub fn io(error: std::io::Error) -> Self {
        tracing::error!(%error, "local I/O failed");
        Self::new(500, "IO_ERROR", "local storage operation failed")
    }
    pub fn unauthorized() -> Self {
        Self::new(
            401,
            "UNAUTHORIZED",
            "authentication required or session expired",
        )
    }
    pub fn forbidden() -> Self {
        Self::new(403, "FORBIDDEN", "permission denied")
    }
    pub fn conflict(code: &str, message: &str) -> Self {
        Self::new(409, code, message)
    }
    pub fn limit(message: &str) -> Self {
        Self::new(429, "RESOURCE_LIMIT_EXCEEDED", message)
    }
    pub fn unsupported() -> Self {
        Self::new(
            501,
            "UNSUPPORTED_OPERATION",
            "operation is not supported by this build",
        )
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ApiError {}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(json!({"error":self}))).into_response()
    }
}

/// Preserve Axum query parsing, but never expose its plain-text rejection body.
pub(crate) struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|error| ApiError::invalid(error.body_text()))
    }
}

/// Invalid client path values are 400; missing routing state remains a 500.
pub(crate) struct ApiPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Path::<T>::from_request_parts(parts, state)
            .await
            .map(|Path(value)| Self(value))
            .map_err(|error| {
                if error.status().is_server_error() {
                    tracing::error!(%error, "request path extraction failed");
                    ApiError::new(500, "INTERNAL_ERROR", "request routing failed")
                } else {
                    ApiError::invalid(error.body_text())
                }
            })
    }
}

/// Use Axum's bounded byte extractor so both fixed and chunked bodies report
/// 413 on overflow, without relying on Content-Length or collecting unbounded data.
pub(crate) async fn read_body(mut request: Request, limit: usize) -> Result<Bytes, ApiError> {
    DefaultBodyLimit::max(limit).apply(&mut request);
    Bytes::from_request(request, &()).await.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::new(413, "PAYLOAD_TOO_LARGE", "request body exceeds limit")
        } else {
            ApiError::invalid("failed to read request body")
        }
    })
}

pub(crate) async fn not_found() -> ApiError {
    ApiError::new(404, "NOT_FOUND", "route not found")
}

pub(crate) async fn method_not_allowed() -> ApiError {
    // Axum adds the matched route's Allow header to this fallback response.
    ApiError::new(
        405,
        "METHOD_NOT_ALLOWED",
        "method not allowed for this route",
    )
}

pub(crate) fn is_api_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}
