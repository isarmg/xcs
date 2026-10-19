//! Static asset admission, validators and the reserved API namespace.
use crate::{ApiError, AppState};
use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) async fn not_found() -> ApiError {
    ApiError::new(404, "NOT_FOUND", "route not found")
}

pub(crate) fn fallback(directory: PathBuf) -> Router {
    Router::new()
        .fallback_service(
            tower_http::services::ServeDir::new(directory)
                .append_index_html_on_directories(true)
                .precompressed_gzip()
                .not_found_service(Router::new().fallback(not_found)),
        )
        .layer(middleware::from_fn(reserve_api_namespace))
}

async fn reserve_api_namespace(request: Request, next: Next) -> Response {
    if is_api_path(request.uri().path()) {
        return not_found().await.into_response();
    }
    next.run(request).await
}

fn is_api_path(path: &str) -> bool {
    path == "/v1" || path.starts_with("/v1/")
}

pub(crate) fn static_etags(root: &Path) -> Result<BTreeMap<String, String>, ApiError> {
    fn walk(
        base: &Path,
        directory: &Path,
        result: &mut BTreeMap<String, String>,
    ) -> Result<(), ApiError> {
        for entry in std::fs::read_dir(directory).map_err(ApiError::io)? {
            let entry = entry.map_err(ApiError::io)?;
            let metadata = entry.path().symlink_metadata().map_err(ApiError::io)?;
            if metadata.file_type().is_symlink() {
                return Err(ApiError::invalid(
                    "web assets must not contain symbolic links",
                ));
            }
            if metadata.is_dir() {
                walk(base, &entry.path(), result)?;
            } else if metadata.is_file() {
                if result.len() >= 4096 || metadata.len() > 64 * 1024 * 1024 {
                    return Err(ApiError::invalid("web assets exceed admission limits"));
                }
                let relative = entry
                    .path()
                    .strip_prefix(base)
                    .map_err(ApiError::invalid)?
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative.ends_with(".gz") {
                    continue;
                }
                let bytes = std::fs::read(entry.path()).map_err(ApiError::io)?;
                let etag = format!("W/\"{}\"", blake3::hash(&bytes).to_hex());
                result.insert(format!("/{relative}"), etag.clone());
                if relative == "index.html" {
                    result.insert("/".into(), etag);
                }
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result)?;
    Ok(result)
}

pub(crate) async fn static_validation(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // Reserved API paths never use static validators, even when an accidental
    // same-named file exists below the configured asset directory.
    let etag = if is_api_path(request.uri().path()) {
        None
    } else {
        state.static_etags.get(request.uri().path()).cloned()
    };
    if matches!(*request.method(), Method::GET | Method::HEAD) {
        if let Some(etag) = &etag {
            if request
                .headers()
                .get(header::IF_NONE_MATCH)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| {
                    value
                        .split(',')
                        .any(|candidate| candidate.trim() == etag || candidate.trim() == "*")
                })
            {
                let mut response = StatusCode::NOT_MODIFIED.into_response();
                response
                    .headers_mut()
                    .insert(header::ETAG, HeaderValue::from_str(etag).unwrap());
                response
                    .headers_mut()
                    .insert(header::VARY, HeaderValue::from_static("accept-encoding"));
                return response;
            }
        }
    }
    let mut response = next.run(request).await;
    if response.status().is_success() {
        if let Some(etag) = etag {
            response
                .headers_mut()
                .insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
        }
    }
    response
}
