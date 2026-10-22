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
    fn upload(&self, _: &Identity, _: &str, _: &str, _: &str, _: &Path) -> Result<Value, ApiError> {
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
    let router = crate::router(probe.clone(), auth, ServerConfig::loopback(&auth_root)).unwrap();
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
