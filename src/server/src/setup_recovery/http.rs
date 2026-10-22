//! Thin HTTP admission. Lifecycle ownership and credential installation remain
//! in ManagementBackend, so client disconnects cannot release setup ownership.
use crate::{ApiError, AppState, auth::Identity, http_boundary};
use http_routing::{
    Json, Router,
    extract::{Request, State},
    routing::{get, post},
};
use json_codec::{Value, json};

pub(crate) fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/setup/status", get(status))
        .route("/api/setup/initialize", post(initialize))
        .route("/api/setup/preset-preview", post(preset_preview))
        .route("/api/setup/initialize-preset", post(initialize_preset))
        .route(
            "/api/setup/complete-initialization",
            post(complete_initialization),
        )
        .route("/api/setup/restore-system", post(bootstrap_restore))
}
pub(crate) fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/management/offline-check", post(offline_check))
        .route("/api/management/restore-system", post(restore))
        .route("/api/management/recovery-preview", post(recovery_preview))
        .route("/api/management/recovery-commit", post(recovery_commit))
}
fn header(request: &Request, name: &'static str) -> Result<String, ApiError> {
    if request.headers().get_all(name).iter().count() != 1 {
        return Err(ApiError::unauthorized());
    }
    request
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .ok_or_else(ApiError::unauthorized)
}
fn csrf(token: &str) -> String {
    blake3::derive_key("zxs bootstrap csrf v1", token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn bootstrap_request(
    state: &AppState,
    request: &Request,
    changing: bool,
) -> Result<String, ApiError> {
    crate::http_auth::setup_admission(state, request)?;
    if request.uri().query().is_some() {
        return Err(ApiError::invalid(
            "setup credentials must not appear in a URL",
        ));
    }
    if request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|v| v != "same-origin")
    {
        return Err(ApiError::forbidden());
    }
    crate::http_auth::same_origin(request.headers(), &state.config, changing)?;
    let token = header(request, "x-zxs-bootstrap")?;
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ApiError::unauthorized());
    }
    if changing {
        let supplied = header(request, "x-csrf-token")?;
        let expected = csrf(&token);
        if supplied.len() != expected.len()
            || supplied
                .bytes()
                .zip(expected.bytes())
                .fold(0u8, |n, (a, b)| n | (a ^ b))
                != 0
        {
            return Err(ApiError::forbidden());
        }
    }
    Ok(token)
}
async fn dispatch(
    state: AppState,
    action: &'static str,
    input: Value,
    bootstrap: Option<String>,
    identity: Option<Identity>,
) -> Result<Value, ApiError> {
    let permit = state
        .management_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("management operation capacity reached"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        state
            .backend
            .setup_operation(action, input, bootstrap.as_deref(), identity.as_ref())
    })
    .await
    .map_err(ApiError::join)?
}
async fn body(request: Request) -> Result<Value, ApiError> {
    if !request
        .headers()
        .get(http_routing::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim() == "application/json")
        })
    {
        return Err(ApiError::new(
            415,
            "UNSUPPORTED_MEDIA_TYPE",
            "application/json is required",
        ));
    }
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        http_boundary::read_body(request, super::MAX_SETUP_BODY_BYTES),
    )
    .await
    .map_err(|_| ApiError::new(408, "REQUEST_TIMEOUT", "setup request body timed out"))??;
    storage_contract::strict_json(&bytes).map_err(ApiError::invalid)
}
async fn status(State(state): State<AppState>, request: Request) -> Result<Json<Value>, ApiError> {
    let token = bootstrap_request(&state, &request, false)?;
    let csrf_token = csrf(&token);
    let mut value = dispatch(state, "status", json!({}), Some(token), None).await?;
    value
        .as_object_mut()
        .ok_or_else(|| ApiError::new(500, "INTERNAL_ERROR", "invalid setup status"))?
        .insert("csrf_token".into(), Value::String(csrf_token));
    Ok(Json(value))
}
async fn public_mutation(
    state: AppState,
    request: Request,
    action: &'static str,
) -> Result<Json<Value>, ApiError> {
    let token = bootstrap_request(&state, &request, true)?;
    let input = body(request).await?;
    dispatch(state, action, input, Some(token), None)
        .await
        .map(Json)
}
async fn initialize(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    public_mutation(state, request, "initialize").await
}
async fn preset_preview(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    public_mutation(state, request, "preset_preview").await
}
async fn initialize_preset(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    public_mutation(state, request, "initialize_preset").await
}
async fn complete_initialization(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    public_mutation(state, request, "complete_initialization").await
}
async fn bootstrap_restore(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    public_mutation(state, request, "restore_system").await
}
async fn protected_mutation(
    state: AppState,
    request: Request,
    action: &'static str,
) -> Result<Json<Value>, ApiError> {
    let identity = request
        .extensions()
        .get::<Identity>()
        .cloned()
        .ok_or_else(ApiError::unauthorized)?;
    if !identity.admin {
        return Err(ApiError::forbidden());
    }
    crate::http_auth::setup_admission(&state, &request)?;
    // The ordinary authorize middleware has checked the session and CSRF.
    let input = body(request).await?;
    dispatch(state, action, input, None, Some(identity))
        .await
        .map(Json)
}
async fn offline_check(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    protected_mutation(state, request, "offline_check").await
}
async fn restore(State(state): State<AppState>, request: Request) -> Result<Json<Value>, ApiError> {
    protected_mutation(state, request, "restore_system").await
}
async fn recovery_preview(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    protected_mutation(state, request, "recovery_preview").await
}
async fn recovery_commit(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    protected_mutation(state, request, "recovery_commit").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, BackendAction, Content, ServerConfig, auth::AuthStore};
    use http_routing::{
        body::{Body, to_bytes},
        http::{Request as HttpRequest, StatusCode},
    };
    use service_stack::ServiceExt;
    use std::os::unix::fs::PermissionsExt;
    use std::{
        collections::BTreeMap,
        path::Path,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    struct Probe {
        service: super::super::SetupRecovery,
        mutations: AtomicUsize,
    }
    impl Backend for Probe {
        fn setup_operation(
            &self,
            action: &'static str,
            _: Value,
            bootstrap: Option<&str>,
            _: Option<&Identity>,
        ) -> Result<Value, ApiError> {
            self.service
                .authorize_bootstrap(bootstrap.ok_or_else(ApiError::unauthorized)?)?;
            if action == "status" {
                return json_codec::to_value(self.service.status()).map_err(ApiError::invalid);
            }
            self.mutations.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"accepted":true,"restart_required":false,"recovering":true}))
        }
        fn call(&self, _: BackendAction, _: &Identity, _: Value) -> Result<Value, ApiError> {
            Err(ApiError::unsupported())
        }
        fn upload(
            &self,
            _: &Identity,
            _: &str,
            _: &str,
            _: &str,
            _: &Path,
        ) -> Result<Value, ApiError> {
            Err(ApiError::unsupported())
        }
        fn content(&self, _: &Identity, _: &str) -> Result<Content, ApiError> {
            Err(ApiError::unsupported())
        }
    }
    fn app() -> (tempfile::TempDir, Router, Arc<Probe>) {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("system");
        let management = temp.path().join("management");
        std::fs::create_dir(&data).unwrap();
        std::fs::create_dir(&management).unwrap();
        let mut disks = BTreeMap::new();
        for id in ["one", "two", "three"] {
            let path = temp.path().join(id);
            std::fs::create_dir(&path).unwrap();
            disks.insert(id.into(), path);
        }
        let key = management.join("bootstrap.key");
        std::fs::write(&key, TOKEN).unwrap();
        std::fs::set_permissions(key, std::fs::Permissions::from_mode(0o600)).unwrap();
        let probe = Arc::new(Probe {
            service: super::super::SetupRecovery::new(data, management, disks).unwrap(),
            mutations: AtomicUsize::new(0),
        });
        let auth_root = temp.path().join("http-auth");
        let auth = AuthStore::initialize(&auth_root, "admin", "long-test-password").unwrap();
        let router =
            crate::router(probe.clone(), auth, ServerConfig::loopback(&auth_root)).unwrap();
        (temp, router, probe)
    }
    fn request(method: &str, path: &str, body: &str) -> http_routing::http::request::Builder {
        let _ = body;
        HttpRequest::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:7400")
            .header("origin", "http://127.0.0.1:7400")
            .header("content-type", "application/json")
    }
    #[tokio::test]
    async fn setup_http_requires_capability_csrf_origin_and_no_secret_query() {
        let (_temp, app, probe) = app();
        let missing = app
            .clone()
            .oneshot(
                request("GET", "/api/setup/status", "")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
        let query = app
            .clone()
            .oneshot(
                request("GET", "/api/setup/status?bootstrap=secret", "")
                    .header("x-zxs-bootstrap", TOKEN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(query.status(), StatusCode::BAD_REQUEST);
        let missing_csrf = app
            .clone()
            .oneshot(
                request("POST", "/api/setup/initialize", "{}")
                    .header("x-zxs-bootstrap", TOKEN)
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_csrf.status(), StatusCode::UNAUTHORIZED);
        let cross = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/setup/initialize")
                    .header("host", "127.0.0.1:7400")
                    .header("origin", "https://evil.invalid")
                    .header("x-zxs-bootstrap", TOKEN)
                    .header("x-csrf-token", csrf(TOKEN))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross.status(), StatusCode::FORBIDDEN);
        let duplicate = app
            .clone()
            .oneshot(
                request("GET", "/api/setup/status", "")
                    .header("x-zxs-bootstrap", TOKEN)
                    .header("x-zxs-bootstrap", TOKEN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(duplicate.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(probe.mutations.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn setup_http_bounds_body_and_returns_csrf_only_after_authorization() {
        let (_temp, app, probe) = app();
        let status = app
            .clone()
            .oneshot(
                request("GET", "/api/setup/status", "")
                    .header("x-zxs-bootstrap", TOKEN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        assert_eq!(status.headers()["cache-control"], "no-store");
        let bytes = to_bytes(status.into_body(), 65536).await.unwrap();
        let value: Value = json_codec::from_slice(&bytes).unwrap();
        assert_eq!(value["csrf_token"], csrf(TOKEN));
        assert!(!String::from_utf8_lossy(&bytes).contains(TOKEN));
        let large = "x".repeat(super::super::MAX_SETUP_BODY_BYTES + 1);
        let overflow = app
            .clone()
            .oneshot(
                request("POST", "/api/setup/initialize", &large)
                    .header("x-zxs-bootstrap", TOKEN)
                    .header("x-csrf-token", csrf(TOKEN))
                    .body(Body::from(large))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(overflow.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let success = app
            .clone()
            .oneshot(
                request("POST", "/api/setup/initialize", "{}")
                    .header("x-zxs-bootstrap", TOKEN)
                    .header("x-csrf-token", csrf(TOKEN))
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(success.status(), StatusCode::OK);
        assert_eq!(probe.mutations.load(Ordering::SeqCst), 1);
        std::fs::remove_file(probe.service.management_root.join("bootstrap.key")).unwrap();
        let revoked = app
            .clone()
            .oneshot(
                request("GET", "/api/setup/status", "")
                    .header("x-zxs-bootstrap", TOKEN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!revoked.status().is_success());
    }
    #[tokio::test]
    async fn setup_http_rate_limits_before_backend_work() {
        let (_temp, app, probe) = app();
        for _ in 0..10 {
            let response = app
                .clone()
                .oneshot(
                    request("GET", "/api/setup/status", "")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        let limited = app
            .clone()
            .oneshot(
                request("GET", "/api/setup/status", "")
                    .header("x-zxs-bootstrap", TOKEN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(probe.mutations.load(Ordering::SeqCst), 0);
    }
}
