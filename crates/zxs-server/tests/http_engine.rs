//! Exercise the public HTTP boundary against the real durable engine without a socket.
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};
use tower::ServiceExt;
use zxs_contract::{Config, new_id, request_digest};
use zxs_core::{CommitRequest, Engine};
use zxs_server::{
    Backend, ServerConfig,
    auth::{AuthStore, Identity},
    engine_backend::EngineBackend,
};
#[path = "http_engine/backup_namespace.rs"]
mod backup_namespace;
#[path = "http_engine/file_metadata_operations.rs"]
mod file_metadata_operations;

/// Resolve only fixture payloads for deliberate corruption. Production reads
/// continue through the engine's verified replica and read-lease interfaces.
fn fixture_payload(version_directory: &Path, state: &zxs_contract::State) -> std::path::PathBuf {
    let version: zxs_contract::FileVersion =
        serde_json::from_slice(&std::fs::read(version_directory.join("version.json")).unwrap())
            .unwrap();
    let Some(storage_name) = version.storage_name else {
        return version_directory
            .join("content")
            .join(version.creation_name);
    };
    let history = version_directory.parent().unwrap().parent().unwrap();
    let stable = history.join("file-containers").join(&version.file_id);
    let named = if let Some(location) = state.file_locations.get(&version.file_id) {
        stable.join("namespace").join(location)
    } else {
        stable.join(
            state
                .file_names
                .get(&version.file_id)
                .unwrap_or(&version.creation_name),
        )
    };
    named.join("versions").join(storage_name)
}

async fn send(
    app: &Router,
    token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(
                    serde_json::to_vec(&body.unwrap_or_else(|| json!({}))).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn success(
    app: &Router,
    token: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Value {
    let action = body
        .as_ref()
        .and_then(|body| body.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let (status, value) = send(app, Some(token), method, path, body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{method} {path} ({action}): {value}"
    );
    value
}
fn application(root: &Path) -> Router {
    let config = Config::development([root.join("d1"), root.join("d2"), root.join("d3")]);
    configured_application(root, config)
}
fn configured_application(root: &Path, config: Config) -> Router {
    let engine = Engine::initialize(&root.join("system"), config).unwrap();
    let backend = Arc::new(EngineBackend::new(engine));
    let auth = backend
        .protect_auth(
            AuthStore::initialize(&root.join("system"), "admin", "integration-test-password")
                .unwrap(),
        )
        .unwrap();
    zxs_server::router(backend, auth, ServerConfig::loopback(&root.join("system"))).unwrap()
}

async fn reopen_after_router_drop(system: &Path) -> Engine {
    // The periodic sampler can hold a temporary strong reference while its
    // current refresh finishes. Dropping the router stops future samples but
    // does not join that in-flight thread. Wait only for its real OS locks;
    // recovery failures and a still-live writer after two seconds remain errors.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match Engine::open(system) {
            Ok(engine) => return engine,
            Err(error) if error.code() == "WRITER_ALREADY_ACTIVE" => {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                assert!(
                    !remaining.is_zero(),
                    "old router retained its writer locks: {error}"
                );
                tokio::time::sleep(remaining.min(std::time::Duration::from_millis(10))).await;
            }
            Err(error) => panic!("fixture recovery failed after router shutdown: {error}"),
        }
    }
}

#[tokio::test]
async fn pool_upload_limit_is_checked_before_creating_spooled_bytes() {
    let root = tempfile::tempdir().unwrap();
    let mut config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    config.max_file_bytes = 16;
    let app = configured_application(root.path(), config);
    let token = login(&app, "admin", "integration-test-password").await;
    let repo = success(
        &app,
        &token,
        "POST",
        "/v1/repositories",
        Some(json!({
            "name":"Small limit", "primary_slot_id":"P01", "target_copies":1,
            "operation_id":new_id("op")
        })),
    )
    .await;
    let workspace = success(
        &app,
        &token,
        "POST",
        "/v1/workspaces",
        Some(json!({"branch_id":repo["main_branch_id"]})),
    )
    .await;
    let bytes = b"too large for this pool";
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/v1/workspaces/{}/files/{}/content?path=large.txt",
                    workspace["workspace_id"].as_str().unwrap(),
                    new_id("entry")
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("x-zxs-logical-size", bytes.len().to_string())
                .header("x-zxs-content-hash", blake3::hash(bytes).to_hex().as_str())
                .body(Body::from(bytes.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        std::fs::read_dir(root.path().join("system/api-spool"))
            .unwrap()
            .count(),
        0
    );
}
async fn login(app: &Router, name: &str, password: &str) -> String {
    let (status, value) = send(
        app,
        None,
        "POST",
        "/v1/auth/login",
        Some(json!({"username":name,"password":password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    value["token"].as_str().unwrap().into()
}

#[tokio::test]
async fn self_password_change_is_csrf_protected_and_restored_from_pool_control_state() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let (status, session) = send(
        &app,
        None,
        "POST",
        "/v1/auth/login",
        Some(json!({
            "username":"admin", "password":"integration-test-password"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = session["token"].as_str().unwrap();
    let cookie = format!("zxs_session={token}");
    let body = json!({"current_password":"integration-test-password", "new_password":"rotated-test-password"});
    for (origin, csrf) in [
        ("http://127.0.0.1:7400", None),
        ("https://foreign.invalid", session["csrf_token"].as_str()),
        ("http://127.0.0.1:7400", Some("wrong-csrf-token")),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/v1/auth/password")
            .header("host", "127.0.0.1:7400")
            .header("origin", origin)
            .header("cookie", &cookie)
            .header("content-type", "application/json");
        if let Some(csrf) = csrf {
            request = request.header("x-csrf-token", csrf);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let mut escalated = body.clone();
    escalated["username"] = "other-user".into();
    escalated["admin"] = true.into();
    assert_eq!(
        send(
            &app,
            Some(token),
            "POST",
            "/v1/auth/password",
            Some(escalated)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/password")
                .header("host", "127.0.0.1:7400")
                .header("origin", "http://127.0.0.1:7400")
                .header("cookie", &cookie)
                .header("x-csrf-token", session["csrf_token"].as_str().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(
        send(&app, Some(token), "GET", "/v1/auth/me", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    drop(app);
    // A lost local auth cache must recover the new password from the pool's
    // authoritative, transactionally published control state.
    std::fs::remove_file(root.path().join("system/auth.json")).unwrap();
    let backend = Arc::new(EngineBackend::new(
        reopen_after_router_drop(&root.path().join("system")).await,
    ));
    let auth = backend.open_auth(&root.path().join("system")).unwrap();
    let app = zxs_server::router(
        backend,
        auth,
        ServerConfig::loopback(&root.path().join("system")),
    )
    .unwrap();
    assert_eq!(
        send(
            &app,
            None,
            "POST",
            "/v1/auth/login",
            Some(json!({"username":"admin", "password":"integration-test-password"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert!(
        !login(&app, "admin", "rotated-test-password")
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn maximum_password_change_accepts_escaped_json_and_rejects_errors_without_logout() {
    let root = tempfile::tempdir().unwrap();
    let system = root.path().join("system");
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let current_password = "\\".repeat(1024);
    let new_password = "\"".repeat(1024);
    let backend = Arc::new(EngineBackend::new(
        Engine::initialize(&system, config).unwrap(),
    ));
    let auth = backend
        .protect_auth(AuthStore::initialize(&system, "admin", &current_password).unwrap())
        .unwrap();
    let app = zxs_server::router(backend, auth, ServerConfig::loopback(&system)).unwrap();
    let (status, session) = send(
        &app,
        None,
        "POST",
        "/v1/auth/login",
        Some(json!({"username":"admin", "password":current_password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = session["token"].as_str().expect("login token missing");
    let csrf = session["csrf_token"].as_str().expect("CSRF token missing");
    let cookie = format!("zxs_session={token}");
    let request = || {
        Request::builder()
            .method("POST")
            .uri("/v1/auth/password")
            .header("host", "127.0.0.1:7400")
            .header("origin", "http://127.0.0.1:7400")
            .header("cookie", &cookie)
            .header("x-csrf-token", csrf)
            .header("content-type", "application/json")
    };
    let rejected = app
        .clone()
        .oneshot(
            request()
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "current_password":"incorrect-current-password",
                        "new_password":new_password
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
    assert!(!rejected.headers().contains_key("set-cookie"));
    assert_eq!(
        send(&app, Some(token), "GET", "/v1/auth/me", None).await.0,
        StatusCode::OK
    );

    // JSON Unicode escapes may use six bytes for each legal password byte.
    // Both decoded passwords stay at the primitive policy's 1024-byte limit.
    let encoded = format!(
        "{{\"current_password\":\"{}\",\"new_password\":\"{}\"}}",
        r"\u005c".repeat(1024),
        r"\u0022".repeat(1024)
    );
    assert!(encoded.len() > 12 * 1024 && encoded.len() < 16 * 1024);
    let changed = app
        .clone()
        .oneshot(request().body(Body::from(encoded)).unwrap())
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::OK);
    assert_eq!(
        send(&app, Some(token), "GET", "/v1/auth/me", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &app,
            None,
            "POST",
            "/v1/auth/login",
            Some(json!({"username":"admin", "password":new_password}))
        )
        .await
        .0,
        StatusCode::OK
    );
}

async fn upload(
    app: &Router,
    token: &str,
    workspace: &str,
    item: &str,
    path: &str,
    bytes: &[u8],
) -> Value {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("path", path)
        .finish();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/v1/workspaces/{workspace}/files/{item}/content?{query}"
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("x-zxs-logical-size", bytes.len().to_string())
                .header("x-zxs-content-hash", blake3::hash(bytes).to_hex().as_str())
                .header("x-zxs-upload-generation", "1")
                .body(Body::from(bytes.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap()
}

async fn version_bytes(app: &Router, token: &str, version: &str) -> Vec<u8> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/versions/{version}/content"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    axum::body::to_bytes(response.into_body(), 65536)
        .await
        .unwrap()
        .to_vec()
}

async fn manager_upload(app: &Router, token: &str, path: &str, bytes: &[u8]) -> Value {
    let workspace = success(
        app,
        token,
        "POST",
        "/v1/file-manager/disks/P01/write-sessions",
        Some(json!({"path":path})),
    )
    .await;
    let id = workspace["workspace_id"].as_str().unwrap();
    upload(
        app,
        token,
        id,
        &new_id("entry"),
        workspace["relative_path"].as_str().unwrap(),
        bytes,
    )
    .await;
    let mut body = success(
        app,
        token,
        "POST",
        &format!("/v1/workspaces/{id}/seal"),
        Some(json!({})),
    )
    .await;
    body["message"] = format!("Upload {path}").into();
    body["request_digest"] = request_digest(
        "POST",
        "/v1/workspaces/{workspace}/commit",
        json!({"workspace":id}),
        json!({}),
        &body,
    )
    .unwrap()
    .into();
    success(
        app,
        token,
        "POST",
        &format!("/v1/workspaces/{id}/commit"),
        Some(body),
    )
    .await
}

#[tokio::test]
async fn file_notes_are_authorized_bounded_persistent_and_download_uses_current_container_name() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "original.txt", b"immutable historical bytes").await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let file = listing["entries"][0]["file_id"].as_str().unwrap();
    let version = listing["entries"][0]["version_id"].as_str().unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    success(
        &app,
        &token,
        "POST",
        "/v1/admin/accounts",
        Some(
            json!({"username":"reader","password":"read-only-test-password","repositories":[repo]}),
        ),
    )
    .await;
    success(&app,&token,"PUT",&format!("/v1/repositories/{repo}/grants"),Some(json!({"grants":{"read":["reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "reader", "read-only-test-password").await;
    assert_eq!(
        send(
            &app,
            Some(&reader),
            "POST",
            &format!("/v1/file-manager/files/{file}/metadata"),
            Some(json!({"note":"cannot edit","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Both metadata endpoints count UTF-8 bytes, reject every control character,
    // and accept an empty note as a deletion of the stored metadata.
    for endpoint in [
        format!("/v1/file-manager/files/{file}/metadata"),
        format!("/v1/file-manager/versions/{version}/metadata"),
    ] {
        for note in [
            "a".repeat(128),
            format!("{}ab", "文".repeat(42)),
            String::new(),
        ] {
            assert_eq!(note.len(), if note.is_empty() { 0 } else { 128 });
            let saved = success(
                &app,
                &token,
                "POST",
                &endpoint,
                Some(json!({"note":note,"operation_id":new_id("op")})),
            )
            .await;
            assert_eq!(saved["result"]["note"], note);
        }
        for note in [
            "a".repeat(129),
            "文".repeat(43),
            "line\nbreak".into(),
            "a\rb".into(),
            "a\tb".into(),
            "a\0b".into(),
            "a\u{7f}b".into(),
        ] {
            assert_eq!(
                send(
                    &app,
                    Some(&token),
                    "POST",
                    &endpoint,
                    Some(json!({"note":note,"operation_id":new_id("op")}))
                )
                .await
                .0,
                StatusCode::BAD_REQUEST
            );
        }
    }
    let metadata = json!({"note":"container note, second line","operation_id":new_id("op")});
    let saved = success(
        &app,
        &token,
        "POST",
        &format!("/v1/file-manager/files/{file}/metadata"),
        Some(metadata.clone()),
    )
    .await;
    assert_eq!(
        success(
            &app,
            &token,
            "POST",
            &format!("/v1/file-manager/files/{file}/metadata"),
            Some(metadata)
        )
        .await["result"],
        saved["result"]
    );
    success(
        &app,
        &token,
        "POST",
        &format!("/v1/file-manager/versions/{version}/metadata"),
        Some(json!({"note":"historical note","operation_id":new_id("op")})),
    )
    .await;
    success(&app,&token,"POST","/v1/file-manager/disks/P01/namespace",Some(json!({"kind":"rename","path":"original.txt","destination":"改名后的文件.txt","operation_id":new_id("op")}))).await;
    let history = success(
        &app,
        &reader,
        "GET",
        &format!("/v1/file-manager/files/{file}/history"),
        None,
    )
    .await;
    assert_eq!(history["note"], "container note, second line");
    assert_eq!(history["versions"][0]["note"], "historical note");
    assert!(
        history["versions"][0]["storage_name"]
            .as_str()
            .unwrap()
            .starts_with("v000001_")
    );
    assert_eq!(history["versions"][0]["legacy"], false);
    assert_eq!(history["versions"][0]["download_name"], "改名后的文件.txt");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/versions/{version}/content"))
                .header("authorization", format!("Bearer {reader}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let disposition = response.headers()["content-disposition"].to_str().unwrap();
    assert!(disposition.contains("filename*=UTF-8''%E6%94%B9"));
    assert!(!disposition.contains("original.txt"));
    assert_eq!(
        &axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap()[..],
        b"immutable historical bytes"
    );
    success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/namespace",
        Some(json!({"kind":"delete","path":"改名后的文件.txt","operation_id":new_id("op")})),
    )
    .await;
    let deleted = success(
        &app,
        &reader,
        "GET",
        &format!("/v1/file-manager/files/{file}/history"),
        None,
    )
    .await;
    assert_eq!(
        deleted["versions"][0]["download_name"], "改名后的文件.txt",
        "deletion preserves the container's last name for historical downloads"
    );
    drop(app);
    let backend = Arc::new(EngineBackend::new(
        reopen_after_router_drop(&root.path().join("system")).await,
    ));
    let auth = backend.open_auth(&root.path().join("system")).unwrap();
    let app = zxs_server::router(
        backend,
        auth,
        ServerConfig::loopback(&root.path().join("system")),
    )
    .unwrap();
    let token = login(&app, "admin", "integration-test-password").await;
    let history = success(
        &app,
        &token,
        "GET",
        &format!("/v1/file-manager/files/{file}/history"),
        None,
    )
    .await;
    assert_eq!(history["note"], "container note, second line");
    assert_eq!(history["versions"][0]["note"], "historical note");
}

#[tokio::test]
async fn file_history_exposes_durable_content_edges_and_preserves_read_permissions() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &admin, "tree.txt", b"first content").await;
    manager_upload(&app, &admin, "sibling.txt", b"independent file").await;
    let listing = success(
        &app,
        &admin,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let entries = listing["entries"].as_array().unwrap();
    let first = entries
        .iter()
        .find(|entry| entry["name"] == "tree.txt")
        .unwrap();
    let file = first["file_id"].as_str().unwrap();
    let original = first["version_id"].as_str().unwrap();
    let sibling = entries
        .iter()
        .find(|entry| entry["name"] == "sibling.txt")
        .unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let history_path = format!("/v1/file-manager/files/{file}/history");

    manager_upload(&app, &admin, "tree.txt", b"second main content").await;
    let main = success(&app, &admin, "GET", &history_path, None).await;
    let main_version = main["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|version| version["current"] == true)
        .unwrap()["version_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(main_version, original);
    let branch = success(
        &app,
        &admin,
        "POST",
        &format!("/v1/file-manager/files/{file}/branches"),
        Some(json!({"source_version_id":original,"name":"from-first","operation_id":new_id("op")})),
    )
    .await;
    let branch_id = branch["result"]["branch_id"].as_str().unwrap();
    let forked = success(&app, &admin, "GET", &history_path, None).await;
    let pending = forked["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["branch_id"] == branch_id)
        .unwrap();
    assert_eq!(pending["version_id"], original);
    assert_eq!(pending["inherited"], true);
    assert!(pending["own_version_id"].is_null());
    assert_eq!(pending["visible_content_state"], "AVAILABLE");
    assert_eq!(pending["materialized"], false);
    assert_eq!(pending["content_state"], "PENDING");
    assert_eq!(pending["source_version_id"], original);
    assert_eq!(forked["versions"].as_array().unwrap().len(), 2);
    success(
        &app,
        &admin,
        "POST",
        &format!("/v1/file-manager/files/{file}/select"),
        Some(json!({"branch_id":branch_id,"operation_id":new_id("op")})),
    )
    .await;
    manager_upload(&app, &admin, "tree.txt", b"new branch content").await;

    success(
        &app, &admin, "POST", "/v1/admin/accounts",
        Some(json!({"username":"tree-reader","password":"tree-reader-test-password","repositories":[repo]})),
    ).await;
    success(
        &app, &admin, "PUT", &format!("/v1/repositories/{repo}/grants"),
        Some(json!({"grants":{"read":["tree-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")})),
    ).await;
    let reader = login(&app, "tree-reader", "tree-reader-test-password").await;
    let history = success(&app, &reader, "GET", &history_path, None).await;
    let versions = history["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 3);
    assert!(
        versions.windows(2).all(|pair| {
            pair[0]["sequence"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                > pair[1]["sequence"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
        }),
        "HTTP history remains newest first, independent of candidate-index order"
    );
    let parent_ids = |id: &str| {
        versions
            .iter()
            .find(|version| version["version_id"] == id)
            .unwrap()["source_version_ids"]
            .clone()
    };
    assert_eq!(parent_ids(original), json!([]));
    assert_eq!(parent_ids(&main_version), json!([original]));
    let newest = versions
        .iter()
        .find(|version| version["current"] == true)
        .unwrap();
    assert_eq!(newest["source_version_ids"], json!([original]));
    assert_ne!(newest["version_id"], main_version);
    assert_eq!(newest["branch_id"], branch_id);
    for version in versions {
        assert_eq!(version["file_id"], file);
        assert_ne!(version["version_id"], sibling["version_id"]);
        assert!(
            version["source_version_ids"]
                .as_array()
                .unwrap()
                .iter()
                .all(|parent| parent != &sibling["version_id"])
        );
    }
    let sibling_history = success(
        &app,
        &reader,
        "GET",
        &format!(
            "/v1/file-manager/files/{}/history",
            sibling["file_id"].as_str().unwrap()
        ),
        None,
    )
    .await;
    assert_eq!(sibling_history["versions"].as_array().unwrap().len(), 1);
    assert_eq!(
        sibling_history["versions"][0]["source_version_ids"],
        json!([])
    );
    assert_eq!(
        send(&app, None, "GET", &history_path, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    for (path, body) in [
        (
            format!("/v1/file-manager/files/{file}/metadata"),
            json!({"note":"forbidden","operation_id":new_id("op")}),
        ),
        (
            format!("/v1/file-manager/files/{file}/branches"),
            json!({"source_version_id":original,"name":"forbidden","operation_id":new_id("op")}),
        ),
    ] {
        assert_eq!(
            send(&app, Some(&reader), "POST", &path, Some(body)).await.0,
            StatusCode::FORBIDDEN
        );
    }
    success(
        &app, &admin, "POST", "/v1/admin/accounts",
        Some(json!({"username":"tree-outsider","password":"tree-outsider-test-password","repositories":[]})),
    ).await;
    let outsider = login(&app, "tree-outsider", "tree-outsider-test-password").await;
    assert_eq!(
        send(&app, Some(&outsider), "GET", &history_path, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    // The same edges are restored from the durable pool, not synthesized by an
    // in-memory test backend or inferred from a render-time list order.
    drop(app);
    let backend = Arc::new(EngineBackend::new(
        reopen_after_router_drop(&root.path().join("system")).await,
    ));
    let auth = backend.open_auth(&root.path().join("system")).unwrap();
    let app = zxs_server::router(
        backend,
        auth,
        ServerConfig::loopback(&root.path().join("system")),
    )
    .unwrap();
    let reader = login(&app, "tree-reader", "tree-reader-test-password").await;
    assert_eq!(
        success(&app, &reader, "GET", &history_path, None).await,
        history
    );

    // This small store uses the scan path. An unavailable query cache must not
    // change API compatibility or ACLs. Large-store indexed fallback is covered
    // by the core routing/corruption tests, not by this fixture.
    let outsider = login(&app, "tree-outsider", "tree-outsider-test-password").await;
    let index = root.path().join("system/index.sqlite");
    let saved_index = root.path().join("system/index.saved");
    std::fs::rename(&index, &saved_index).unwrap();
    std::fs::create_dir(&index).unwrap();
    assert_eq!(
        success(&app, &reader, "GET", &history_path, None).await,
        history,
        "an unavailable cache must preserve scan-path versions, order, and edges"
    );
    assert_eq!(
        send(&app, Some(&outsider), "GET", &history_path, None)
            .await
            .0,
        StatusCode::FORBIDDEN,
        "an unavailable cache must not bypass repository read permission"
    );
    std::fs::remove_dir(&index).unwrap();
    std::fs::rename(&saved_index, &index).unwrap();
}

#[tokio::test]
async fn file_manager_exposes_disk_directories_and_independent_file_branches() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    let capabilities = success(&app, &token, "GET", "/v1/capabilities", None).await;
    assert_eq!(capabilities["full_branches"], false);
    assert_eq!(capabilities["branch_creation"], "record-until-write");
    assert_eq!(
        send(&app, None, "GET", "/v1/file-manager/disks", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let disks = success(&app, &token, "GET", "/v1/file-manager/disks", None).await;
    assert_eq!(disks["disks"][0]["slot_id"], "P01");
    assert_eq!(disks["disks"][0]["writable"], true);
    assert_eq!(disks["disks"][1]["writable"], false);
    assert!(
        success(
            &app,
            &token,
            "GET",
            "/v1/file-manager/disks/P01/entries",
            None
        )
        .await["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mkdir = json!({"kind":"mkdir","path":"documents","operation_id":new_id("op")});
    let first = success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/namespace",
        Some(mkdir.clone()),
    )
    .await;
    assert_eq!(
        success(
            &app,
            &token,
            "POST",
            "/v1/file-manager/disks/P01/namespace",
            Some(mkdir)
        )
        .await["result"],
        first["result"]
    );
    manager_upload(&app, &token, "documents/one.txt", b"main file").await;
    manager_upload(&app, &token, "documents/two.txt", b"untouched sibling").await;
    let entries = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=documents",
        None,
    )
    .await;
    let real_source_commit = entries["context"]["base_head"].clone();
    let files = entries["entries"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    let one = files.iter().find(|e| e["name"] == "one.txt").unwrap();
    let two = files.iter().find(|e| e["name"] == "two.txt").unwrap();
    let file = one["file_id"].as_str().unwrap();
    let create_body =
        json!({"source_version_id":one["version_id"],"name":"draft","operation_id":new_id("op")});
    let create_path = format!("/v1/file-manager/files/{file}/branches");
    let branch = success(
        &app,
        &token,
        "POST",
        &create_path,
        Some(create_body.clone()),
    )
    .await;
    assert_eq!(branch["result"]["copied_files"], "0");
    assert_eq!(branch["result"]["placeholder_files"], "0");
    assert_eq!(
        success(&app, &token, "POST", &create_path, Some(create_body)).await["result"],
        branch["result"]
    );
    let branch_id = branch["result"]["branch_id"].as_str().unwrap();
    let history_path = format!("/v1/file-manager/files/{file}/history");
    let pending_history = success(&app, &token, "GET", &history_path, None).await;
    assert_eq!(pending_history["versions"].as_array().unwrap().len(), 1);
    let pending_branch = pending_history["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["branch_id"] == branch_id)
        .unwrap();
    assert_eq!(pending_branch["version_id"], one["version_id"]);
    assert_eq!(pending_branch["inherited"], true);
    assert!(pending_branch["own_version_id"].is_null());
    assert_eq!(pending_branch["source_version_id"], one["version_id"]);
    assert_eq!(pending_branch["materialized"], false);
    assert_eq!(pending_branch["current"], false);
    let branch_info = success(
        &app,
        &token,
        "GET",
        &format!("/v1/branches/{branch_id}"),
        None,
    )
    .await;
    assert_eq!(branch_info["content_state"], "PENDING");
    for suffix in [
        "tree".to_string(),
        format!(
            "tree?commit_id={}",
            branch_info["head_commit_id"].as_str().unwrap()
        ),
    ] {
        let tree = success(
            &app,
            &token,
            "GET",
            &format!("/v1/branches/{branch_id}/{suffix}"),
            None,
        )
        .await;
        let pending_entry = tree["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["file_id"] == file)
            .unwrap();
        assert_eq!(pending_entry["version_id"], one["version_id"]);
        assert_eq!(pending_entry["size"], one["size"]);
    }
    let resolved = success(
        &app,
        &token,
        "POST",
        &format!("/v1/branches/{branch_id}/resolve"),
        Some(json!({"path":"documents/one.txt"})),
    )
    .await;
    assert_eq!(resolved["version"]["version_id"], one["version_id"]);
    assert_eq!(resolved["inherited"], true);
    assert!(resolved["own_version_id"].is_null());
    assert_eq!(resolved["content_state"], "PENDING");
    // A main-directory workspace must retain the file branch it captured,
    // even when two selections happen to refer to the same lineage version.
    let stale = success(
        &app,
        &token,
        "POST",
        "/v1/workspaces",
        Some(json!({"branch_id":entries["context"]["branch_id"]})),
    )
    .await;
    let stale_id = stale["workspace_id"].as_str().unwrap();
    upload(
        &app,
        &token,
        stale_id,
        &new_id("entry"),
        "documents/one.txt",
        b"preserved stale workspace bytes",
    )
    .await;
    success(
        &app,
        &token,
        "POST",
        &format!("/v1/file-manager/files/{file}/select"),
        Some(json!({"branch_id":branch_id,"operation_id":new_id("op")})),
    )
    .await;
    let mut stale_commit = success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{stale_id}/seal"),
        Some(json!({})),
    )
    .await;
    stale_commit["message"] = "must not follow a changed selected branch".into();
    stale_commit["request_digest"] = request_digest(
        "POST",
        "/v1/workspaces/{workspace}/commit",
        json!({"workspace":stale_id}),
        json!({}),
        &stale_commit,
    )
    .unwrap()
    .into();
    let (status, rejected) = send(
        &app,
        Some(&token),
        "POST",
        &format!("/v1/workspaces/{stale_id}/commit"),
        Some(stale_commit),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(rejected["error"]["code"], "FILE_BRANCH_CONFLICT");
    success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{stale_id}/abort"),
        Some(json!({})),
    )
    .await;
    let pending_listing = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=documents",
        None,
    )
    .await;
    let pending_entry = pending_listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["file_id"] == file)
        .unwrap();
    assert_eq!(pending_entry["version_id"], one["version_id"]);
    assert_eq!(pending_entry["materialized"], false);
    assert_eq!(pending_entry["inherited"], true);
    assert!(pending_entry["own_version_id"].is_null());
    assert_eq!(pending_entry["size"], one["size"]);
    assert_eq!(pending_entry["source_version_id"], one["version_id"]);
    assert_eq!(pending_entry["selected_branch_id"], branch_id);
    let pending_backup = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/B01/entries?path=P01%2Fdocuments",
        None,
    )
    .await;
    assert_eq!(pending_backup["entries"].as_array().unwrap().len(), 2);
    assert_eq!(
        pending_backup["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["file_id"] == file)
            .unwrap()["version_id"],
        one["version_id"]
    );
    assert_eq!(
        version_bytes(&app, &token, one["version_id"].as_str().unwrap()).await,
        b"main file"
    );
    let before_aborted = success(&app, &token, "GET", &history_path, None).await;
    let untouched = success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/write-sessions",
        Some(json!({"path":"documents/one.txt"})),
    )
    .await;
    success(
        &app,
        &token,
        "POST",
        &format!(
            "/v1/workspaces/{}/abort",
            untouched["workspace_id"].as_str().unwrap()
        ),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        success(&app, &token, "GET", &history_path, None).await,
        before_aborted
    );
    drop(app);
    let backend = Arc::new(EngineBackend::new(
        reopen_after_router_drop(&root.path().join("system")).await,
    ));
    let auth = backend.open_auth(&root.path().join("system")).unwrap();
    let app = zxs_server::router(
        backend,
        auth,
        ServerConfig::loopback(&root.path().join("system")),
    )
    .unwrap();
    let token = login(&app, "admin", "integration-test-password").await;
    assert_eq!(
        success(&app, &token, "GET", &history_path, None).await,
        before_aborted
    );
    manager_upload(&app, &token, "documents/one.txt", b"selected file draft").await;
    let entries = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=documents",
        None,
    )
    .await;
    assert_eq!(
        entries["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "two.txt")
            .unwrap()["version_id"],
        two["version_id"]
    );
    let history = success(
        &app,
        &token,
        "GET",
        &format!("/v1/file-manager/files/{file}/history"),
        None,
    )
    .await;
    assert_eq!(history["selected_branch_id"], branch_id);
    assert_eq!(history["branches"].as_array().unwrap().len(), 2);
    assert_eq!(history["versions"].as_array().unwrap().len(), 2);
    let materialized = history["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["branch_id"] == branch_id)
        .unwrap();
    assert_eq!(materialized["materialized"], true);
    assert_eq!(materialized["inherited"], false);
    assert_eq!(materialized["own_version_id"], materialized["version_id"]);
    assert!(materialized["source_version_id"].is_null());
    assert!(materialized["version_id"].is_string());
    // Historical registration views keep the parent version after the branch
    // gets its own content, and never claim that parent as an owned version.
    let pending_commit = branch_info["head_commit_id"].as_str().unwrap();
    let old_pending = success(
        &app,
        &token,
        "GET",
        &format!("/v1/branches/{branch_id}/tree?commit_id={pending_commit}"),
        None,
    )
    .await;
    let old_entry = old_pending["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["file_id"] == file)
        .unwrap();
    assert_eq!(old_entry["version_id"], one["version_id"]);
    assert_eq!(old_entry["size"], one["size"]);
    assert_eq!(
        old_pending["pending_file_versions"][file],
        one["version_id"]
    );
    let old_resolved = success(
        &app,
        &token,
        "POST",
        &format!("/v1/branches/{branch_id}/resolve"),
        Some(json!({"path":"documents/one.txt","commit_id":pending_commit})),
    )
    .await;
    assert_eq!(old_resolved["version"]["version_id"], one["version_id"]);
    assert_eq!(old_resolved["inherited"], true);
    assert!(old_resolved["own_version_id"].is_null());
    assert_eq!(old_resolved["source_version_id"], one["version_id"]);
    assert_eq!(old_resolved["content_state"], "PENDING");
    let actual_source = success(
        &app,
        &token,
        "POST",
        &format!("/v1/branches/{branch_id}/resolve"),
        Some(json!({"path":"documents/one.txt","commit_id":real_source_commit})),
    )
    .await;
    assert!(actual_source["version"].is_object());
    assert_eq!(actual_source["version"]["version_id"], one["version_id"]);
    assert!(
        history["versions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["created_at"].is_string())
    );
    assert_eq!(
        send(
            &app,
            Some(&token),
            "POST",
            "/v1/file-manager/disks/B01/write-sessions",
            Some(json!({"path":"P01/documents/one.txt"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let backup = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/B01/entries?path=P01%2Fdocuments",
        None,
    )
    .await;
    assert_eq!(backup["writable"], false);
    assert_eq!(backup["entries"].as_array().unwrap().len(), 2);
    assert_eq!(
        send(
            &app,
            Some(&token),
            "GET",
            "/v1/file-manager/disks/P01/entries?path=..%2F..",
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let events = success(&app, &token, "GET", "/v1/file-manager/history", None).await;
    assert!(
        events["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "file_branch_pending"
                && e["content_state"] == "PENDING"
                && e["inherited"] == true
                && e["own_version_id"].is_null()
                && e["version_id"] == one["version_id"])
    );
    assert!(
        !events["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "delete" && e["path"] == "documents/two.txt")
    );
    let rename = json!({"kind":"rename","path":"documents/two.txt","destination":"documents/renamed.txt","operation_id":new_id("op")});
    success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/namespace",
        Some(rename),
    )
    .await;
    success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/namespace",
        Some(json!({"kind":"delete","path":"documents/renamed.txt","operation_id":new_id("op")})),
    )
    .await;
    let events = success(&app, &token, "GET", "/v1/file-manager/history", None).await;
    assert!(
        events["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "delete" && e["path"] == "documents/renamed.txt")
    );
}

#[tokio::test]
async fn directory_branches_compose_shared_files_and_refuse_partial_selection() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "folder/a.txt", b"A").await;
    manager_upload(&app, &token, "folder/nested/b.txt", b"B").await;
    manager_upload(&app, &token, "folder/c.txt", b"C").await;
    manager_upload(&app, &token, "outside.txt", b"outside").await;
    let parent_folder = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder",
        None,
    )
    .await;
    let parent_nested = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder%2Fnested",
        None,
    )
    .await;
    let parent_a = parent_folder["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "a.txt")
        .unwrap();
    let parent_b = parent_nested["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "b.txt")
        .unwrap();
    let parent_c = parent_folder["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "c.txt")
        .unwrap();
    let before = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let outside = before["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "outside.txt")
        .unwrap()["version_id"]
        .clone();
    let body = json!({"path":"folder","name":"draft","operation_id":new_id("op")});
    let created = success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/directory-branches",
        Some(body.clone()),
    )
    .await;
    assert_eq!(created["result"]["count"], "3");
    assert_eq!(created["result"]["copied_files"], "0");
    assert_eq!(created["result"]["placeholder_files"], "0");
    assert_eq!(created["result"]["materialized"], false);
    assert_eq!(created["result"]["branches"].as_array().unwrap().len(), 3);
    assert_eq!(
        success(
            &app,
            &token,
            "POST",
            "/v1/file-manager/disks/P01/directory-branches",
            Some(body)
        )
        .await["result"],
        created["result"]
    );
    let listing = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder",
        None,
    )
    .await;
    assert_eq!(listing["directory_file_count"], "3");
    assert_eq!(listing["directory_branches_supported"], true);
    assert!(listing["directory_branches"].as_array().unwrap().len() >= 2);
    assert!(
        listing["directory_branches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|branch| branch["current"] == true && branch["materialized"] == true)
    );
    success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/directory-select",
        Some(json!({"path":"folder","name":"draft","operation_id":new_id("op")})),
    )
    .await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder",
        None,
    )
    .await;
    assert!(
        listing["directory_branches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|branch| branch["name"] == "draft" && branch["current"] == true)
    );
    let pending_a = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "a.txt")
        .unwrap();
    assert_eq!(pending_a["materialized"], false);
    assert!(pending_a["version_id"].is_string());
    assert_eq!(pending_a["inherited"], true);
    assert!(pending_a["own_version_id"].is_null());
    assert_eq!(pending_a["source_version_id"], parent_a["version_id"]);
    assert_eq!(pending_a["version_id"], parent_a["version_id"]);
    for (parent, expected) in [
        (parent_a, b"A".as_slice()),
        (parent_b, b"B".as_slice()),
        (parent_c, b"C".as_slice()),
    ] {
        let history = success(
            &app,
            &token,
            "GET",
            &format!(
                "/v1/file-manager/files/{}/history",
                parent["file_id"].as_str().unwrap()
            ),
            None,
        )
        .await;
        assert_eq!(
            history["versions"].as_array().unwrap().len(),
            1,
            "registering and selecting branches never copies content"
        );
        assert_eq!(history["inherited"], true);
        assert_eq!(history["version_id"], parent["version_id"]);
        assert!(history["own_version_id"].is_null());
        assert_eq!(
            history["branches"].as_array().unwrap().len(),
            1,
            "directory registration must reuse each existing shared file branch"
        );
        assert_eq!(
            version_bytes(&app, &token, history["version_id"].as_str().unwrap()).await,
            expected
        );
    }
    manager_upload(
        &app,
        &token,
        "folder/a.txt",
        b"only A owns new branch bytes",
    )
    .await;
    let nested = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder%2Fnested",
        None,
    )
    .await;
    let pending_b = nested["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "b.txt")
        .unwrap();
    assert_eq!(pending_b["materialized"], false);
    assert!(pending_b["version_id"].is_string());
    assert_eq!(pending_b["inherited"], true);
    assert!(pending_b["own_version_id"].is_null());
    let b_history = success(
        &app,
        &token,
        "GET",
        &format!(
            "/v1/file-manager/files/{}/history",
            pending_b["file_id"].as_str().unwrap()
        ),
        None,
    )
    .await;
    assert_eq!(b_history["versions"].as_array().unwrap().len(), 1);
    let a_history = success(
        &app,
        &token,
        "GET",
        &format!(
            "/v1/file-manager/files/{}/history",
            pending_a["file_id"].as_str().unwrap()
        ),
        None,
    )
    .await;
    assert_eq!(a_history["versions"].as_array().unwrap().len(), 2);
    let mixed = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder",
        None,
    )
    .await;
    let written_a = mixed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "a.txt")
        .unwrap();
    let inherited_c = mixed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "c.txt")
        .unwrap();
    assert_eq!(written_a["materialized"], true);
    assert_eq!(written_a["inherited"], false);
    assert_eq!(written_a["own_version_id"], written_a["version_id"]);
    assert_ne!(written_a["version_id"], parent_a["version_id"]);
    assert_eq!(
        version_bytes(&app, &token, written_a["version_id"].as_str().unwrap()).await,
        b"only A owns new branch bytes"
    );
    for (inherited, parent, expected) in [
        (pending_b, parent_b, b"B".as_slice()),
        (inherited_c, parent_c, b"C".as_slice()),
    ] {
        assert_eq!(inherited["materialized"], false);
        assert_eq!(inherited["inherited"], true);
        assert_eq!(inherited["version_id"], parent["version_id"]);
        assert_eq!(inherited["source_version_id"], parent["version_id"]);
        assert_eq!(inherited["visible_content_state"], "AVAILABLE");
        assert!(inherited["own_version_id"].is_null());
        assert_eq!(
            version_bytes(&app, &token, inherited["version_id"].as_str().unwrap()).await,
            expected
        );
    }
    assert_eq!(
        version_bytes(&app, &token, parent_a["version_id"].as_str().unwrap()).await,
        b"A",
        "writing the branch never changes the parent version"
    );
    assert_eq!(
        version_bytes(&app, &token, outside.as_str().unwrap()).await,
        b"outside"
    );
    let after = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    assert_eq!(
        after["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "outside.txt")
            .unwrap()["version_id"],
        outside
    );
    manager_upload(&app, &token, "folder/new.txt", b"new").await;
    success(
        &app,
        &token,
        "POST",
        "/v1/file-manager/disks/P01/directory-select",
        Some(json!({
            "path":"folder","name":"main","operation_id":new_id("op")
        })),
    )
    .await;
    let restored = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder",
        None,
    )
    .await;
    assert!(
        restored["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["name"] != "new.txt"),
        "P1 restores the complete old namespace, without rejecting new files absent from it"
    );
    assert_eq!(
        restored["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == "a.txt")
            .unwrap()["version_id"],
        parent_a["version_id"]
    );
    assert_eq!(
        send(
            &app,
            Some(&token),
            "POST",
            "/v1/file-manager/disks/B01/directory-branches",
            Some(json!({"path":"P01/folder","name":"readonly","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    success(&app,&token,"POST","/v1/repositories",Some(json!({"name":"Earlier collection","primary_slot_id":"P01","target_copies":3,"operation_id":new_id("op")}))).await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    assert_eq!(listing["directory_branches_supported"], false);
    assert_eq!(
        send(
            &app,
            Some(&token),
            "POST",
            "/v1/file-manager/disks/P01/directory-branches",
            Some(json!({"path":"","name":"root partial forbidden","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn durable_http_save_seals_the_real_manifest_and_keeps_original_publication_result() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    let repo=success(&app,&token,"POST","/v1/repositories",Some(json!({"name":"HTTP repository","primary_slot_id":"P01","target_copies":3,"operation_id":new_id("op")}))).await;
    let branch = repo["main_branch_id"].as_str().unwrap();
    let workspace = success(
        &app,
        &token,
        "POST",
        "/v1/workspaces",
        Some(json!({"branch_id":branch})),
    )
    .await;
    let ws = workspace["workspace_id"].as_str().unwrap();
    success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{ws}/namespace-operations"),
        Some(json!({"kind":"mkdir","path":"documents","destination":null})),
    )
    .await;
    let item = new_id("entry");
    let bytes = b"a complete independent full file";
    upload(&app, &token, ws, &item, "documents/file.txt", bytes).await;
    let status = success(
        &app,
        &token,
        "GET",
        &format!("/v1/workspaces/{ws}/files/{item}/upload-status"),
        None,
    )
    .await;
    assert_eq!(status["state"], "COMPLETE");
    assert_eq!(
        status["durable_ranges"][0]["content_hash"],
        blake3::hash(bytes).to_hex().as_str()
    );
    let mut body = success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{ws}/seal"),
        Some(json!({})),
    )
    .await;
    body["message"] = "Save a complete file".into();
    let digest = request_digest(
        "POST",
        "/v1/workspaces/{workspace}/commit",
        json!({"workspace":ws}),
        json!({}),
        &body,
    )
    .unwrap();
    body["request_digest"] = digest.into();
    let sealed = success(
        &app,
        &token,
        "GET",
        &format!("/v1/workspaces/{ws}/files/{item}/upload-status"),
        None,
    )
    .await;
    assert_eq!(sealed["state"], "SEALED");
    assert_eq!(sealed["sealed_manifest_hash"], body["sealed_manifest_hash"]);
    let saved = success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{ws}/commit"),
        Some(body.clone()),
    )
    .await;
    assert_eq!(saved["state"], "completed");
    assert_eq!(saved["operation_kind"], "content_save");
    assert_eq!(saved["result"]["outcome"], "COMMITTED_AND_PROTECTED");
    assert_eq!(saved["result"]["protection_at_publish"], "TARGET_MET");
    assert_eq!(
        saved["result"]["versions"][0]["required_slots"],
        json!(["P01", "B01", "C01"])
    );
    let retry = success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{ws}/commit"),
        Some(body.clone()),
    )
    .await;
    assert_eq!(retry["result"], saved["result"]);
    body["message"] = "A different request may not reuse this operation".into();
    let (status, _) = send(
        &app,
        Some(&token),
        "POST",
        &format!("/v1/workspaces/{ws}/commit"),
        Some(body),
    )
    .await;
    assert!(!status.is_success());
    let version = saved["result"]["versions"][0]["version_id"]
        .as_str()
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/versions/{version}/content"))
                .header("authorization", format!("Bearer {token}"))
                .header("range", "bytes=2-9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        &axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap()[..],
        &bytes[2..10]
    );
    let checksums = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/versions/{version}/checksums"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(checksums.status(), StatusCode::OK);
    assert!(
        !axum::body::to_bytes(checksums.into_body(), 65536)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn repository_permissions_apply_to_direct_content_graph_and_write_paths() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    let repo=success(&app,&admin,"POST","/v1/repositories",Some(json!({"name":"Private repository","primary_slot_id":"P01","target_copies":1,"operation_id":new_id("op")}))).await;
    success(&app,&admin,"POST","/v1/admin/accounts",Some(json!({"username":"reader","password":"reader-test-password","admin":false,"repositories":[],"disabled":false}))).await;
    let reader = login(&app, "reader", "reader-test-password").await;
    for path in [
        format!(
            "/v1/repositories/{}",
            repo["repository_id"].as_str().unwrap()
        ),
        format!(
            "/v1/branches/{}/tree",
            repo["main_branch_id"].as_str().unwrap()
        ),
        format!(
            "/v1/history-trees/{}/graph",
            repo["history_tree_id"].as_str().unwrap()
        ),
    ] {
        assert_eq!(
            send(&app, Some(&reader), "GET", &path, None).await.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        send(
            &app,
            Some(&reader),
            "POST",
            "/v1/workspaces",
            Some(json!({"branch_id":repo["main_branch_id"]}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(&app, Some(&reader), "GET", "/v1/admin/accounts", None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    success(&app,&admin,"POST","/v1/admin/accounts",Some(json!({"username":"reader","password":null,"admin":false,"repositories":[],"disabled":true}))).await;
    assert_eq!(
        send(&app, Some(&reader), "GET", "/v1/auth/me", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn historical_plan_publishes_atomically_and_management_rules_use_generation_checks() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    let repo = success(
        &app,
        &token,
        "POST",
        "/v1/repositories",
        Some(json!({
            "name":"Historical HTTP", "primary_slot_id":"P01", "target_copies":1,
            "operation_id":new_id("op")
        })),
    )
    .await;
    let repository = repo["repository_id"].as_str().unwrap();
    let branch = repo["main_branch_id"].as_str().unwrap();
    let history = repo["history_tree_id"].as_str().unwrap();
    let original = success(&app, &token, "GET", &format!("/v1/branches/{branch}"), None).await;
    let source = original["head_commit_id"].as_str().unwrap();
    let policy = new_id("policy");
    let policy_path = format!("/v1/policies/{policy}");
    let policy_body = json!({"repository_id":repository,"path_prefix":"","target_copies":3,
        "expected_generation":"0","operation_id":new_id("op")});
    success(&app, &token, "PUT", &policy_path, Some(policy_body.clone())).await;
    assert_eq!(
        success(&app, &token, "GET", &policy_path, None).await["target_copies"],
        3
    );
    let mut stale = policy_body;
    stale["operation_id"] = new_id("op").into();
    assert_eq!(
        send(&app, Some(&token), "PUT", &policy_path, Some(stale))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let rule = new_id("rule");
    let rule_path = format!("/v1/primary-placement-rules/{rule}");
    success(
        &app,
        &token,
        "PUT",
        &rule_path,
        Some(json!({
            "repository_id":repository,"path_prefix":"","primary_slot_id":"P01",
            "expected_generation":"0","operation_id":new_id("op")
        })),
    )
    .await;
    assert_eq!(
        success(&app, &token, "GET", &rule_path, None).await["primary_slot_id"],
        "P01"
    );
    let workspace = success(
        &app,
        &token,
        "POST",
        "/v1/workspaces",
        Some(json!({
            "branch_id":branch, "source_commit_id":source
        })),
    )
    .await;
    let ws = workspace["workspace_id"].as_str().unwrap();
    upload(
        &app,
        &token,
        ws,
        &new_id("entry"),
        "historical.txt",
        b"atomic historical full save",
    )
    .await;
    let plan = success(
        &app,
        &token,
        "POST",
        &format!("/v1/history-trees/{history}/historical-save-plans"),
        Some(json!({
            "source_commit_id":source,"name":"Independent historical branch","workspace_id":ws,
            "primary_slot_id":null,"target_copies":null
        })),
    )
    .await;
    let plan_id = plan["plan_id"].as_str().unwrap();
    let operation = new_id("op");
    let body = json!({"plan_digest":plan["digest"],"operation_id":operation});
    let result = success(
        &app,
        &token,
        "POST",
        &format!("/v1/plans/{plan_id}/commit"),
        Some(body.clone()),
    )
    .await;
    assert_eq!(result["commit_state"], "PUBLISHED");
    assert_eq!(
        success(
            &app,
            &token,
            "POST",
            &format!("/v1/plans/{plan_id}/commit"),
            Some(body)
        )
        .await,
        result
    );
    assert_eq!(
        success(&app, &token, "GET", &format!("/v1/branches/{branch}"), None).await["head_commit_id"],
        source
    );
    let independent = result["result"]["branch_id"].as_str().unwrap();
    let tree = success(
        &app,
        &token,
        "GET",
        &format!("/v1/branches/{independent}/tree"),
        None,
    )
    .await;
    let version = tree["entries"][0]["version_id"].as_str().unwrap();
    let placement = success(
        &app,
        &token,
        "GET",
        &format!("/v1/versions/{version}/placement"),
        None,
    )
    .await;
    assert_eq!(placement["target_copies"], 3);
    let binding_path = format!("/v1/repositories/{repository}/smb-binding");
    let binding = success(&app, &token, "GET", &binding_path, None).await;
    success(&app, &token, "PUT", &binding_path, Some(json!({
        "branch_id":independent,"enabled":true,"expected_history_generation":binding["history_generation"],
        "operation_id":new_id("op")
    }))).await;
    assert_eq!(
        success(&app, &token, "GET", &binding_path, None).await["branch_id"],
        independent
    );
}

fn capture_wire(name: &str, wire: &Value) {
    if let Some(directory) = std::env::var_os("ZXS_TEST_WIRE_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(name),
            serde_json::to_vec_pretty(wire).unwrap(),
        )
        .unwrap();
    }
}

fn operation_identity() -> Identity {
    Identity {
        principal: "admin".into(),
        admin: true,
        repositories: Default::default(),
    }
}

#[test]
fn recovered_permanent_loss_returns_degraded_save_wire_with_frozen_unavailable_slots() {
    let root = tempfile::tempdir().unwrap();
    let system = root.path().join("system");
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let mut engine = Engine::initialize(&system, config).unwrap();
    let repo = engine
        .create_repository("Lost candidate", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, "admin")
        .unwrap();
    let lost = engine
        .stage_bytes(
            &workspace.workspace_id,
            "lost.txt",
            b"permanently lost original payload",
        )
        .unwrap();
    engine
        .stage_bytes(
            &workspace.workspace_id,
            "healthy.txt",
            b"separately retained full payload",
        )
        .unwrap();
    let operation = new_id("op");
    engine.inject_fault_once("after_accept_2");
    assert!(
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: operation.clone(),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "fault before publication".into(),
                request_digest: None,
            })
            .is_err()
    );
    // All complete copies plus the private staging source are deliberately lost.
    for disk in &engine.config().disks {
        let directory = disk
            .root
            .join("histories")
            .join(&repo.history_tree_id)
            .join("versions");
        for version in std::fs::read_dir(directory).unwrap() {
            let directory = version.unwrap().path();
            let metadata: zxs_contract::FileVersion =
                serde_json::from_slice(&std::fs::read(directory.join("version.json")).unwrap())
                    .unwrap();
            if metadata.creation_name != "lost.txt" {
                continue;
            }
            let path = fixture_payload(&directory, engine.state());
            if path.exists() {
                std::fs::remove_file(path).unwrap();
            }
        }
    }
    std::fs::remove_file(lost.source_path).unwrap();
    drop(engine);
    let mut engine = Engine::open_for_recovery(&system).unwrap();
    let plan = engine
        .prepare_degraded_recovery(
            &operation,
            "admin",
            "All fixed copies and the independent staged source were permanently lost",
        )
        .unwrap();
    engine
        .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "admin", true)
        .unwrap();
    let backend = EngineBackend::new(engine);
    let wire = backend
        .call(
            zxs_server::BackendAction::Operation,
            &operation_identity(),
            json!({"operation_id":operation}),
        )
        .unwrap();
    assert_eq!(wire["state"], "completed");
    assert_eq!(wire["commit_state"], "PUBLISHED_DEGRADED");
    assert_eq!(wire["result"]["outcome"], "COMMITTED_DEGRADED");
    assert_eq!(wire["result"]["protection_at_publish"], "BELOW_TARGET");
    assert_eq!(wire["current_health"]["state"], "UNAVAILABLE");
    let versions = wire["result"]["versions"].as_array().unwrap();
    assert_eq!(
        versions
            .iter()
            .filter(|version| version["slots"]
                .as_array()
                .unwrap()
                .iter()
                .all(|slot| slot["state"] == "UNAVAILABLE"))
            .count(),
        1
    );
    assert_eq!(
        versions
            .iter()
            .filter(|version| version["slots"]
                .as_array()
                .unwrap()
                .iter()
                .all(|slot| slot["state"] == "COMPLETE_VERIFIED"))
            .count(),
        1
    );
    capture_wire("degraded-save-operation.json", &wire);
}

#[test]
fn damage_after_normal_publication_keeps_original_target_met_in_save_wire() {
    let root = tempfile::tempdir().unwrap();
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
    let repo = engine
        .create_repository("Published evidence", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, "admin")
        .unwrap();
    engine
        .stage_bytes(
            &workspace.workspace_id,
            "published.txt",
            b"normal full protected publication",
        )
        .unwrap();
    let operation = new_id("op");
    let saved = engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: operation.clone(),
            principal: "admin".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "normal publication".into(),
            request_digest: None,
        })
        .unwrap();
    let version = saved.result["version_protection"][0]["version_id"]
        .as_str()
        .unwrap();
    let primary = engine
        .config()
        .disks
        .iter()
        .find(|disk| disk.slot_id == "P01")
        .unwrap();
    let corrupted = fixture_payload(
        &primary
            .root
            .join("histories")
            .join(&repo.history_tree_id)
            .join("versions")
            .join(version),
        engine.state(),
    );
    std::fs::remove_file(&corrupted).unwrap();
    std::fs::write(&corrupted, b"corruption after successful publication").unwrap();
    let wire = EngineBackend::new(engine)
        .call(
            zxs_server::BackendAction::Operation,
            &operation_identity(),
            json!({"operation_id":operation}),
        )
        .unwrap();
    assert_eq!(wire["commit_state"], "PUBLISHED");
    assert_eq!(wire["result"]["outcome"], "COMMITTED_AND_PROTECTED");
    assert_eq!(wire["result"]["protection_at_publish"], "TARGET_MET");
    assert_eq!(wire["current_health"]["state"], "DEGRADED");
    assert!(
        wire["result"]["versions"][0]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .all(|slot| slot["state"] == "COMPLETE_VERIFIED")
    );
    capture_wire("damaged-published-save-operation.json", &wire);
}

#[tokio::test]
async fn default_directory_branch_is_shared_authorized_and_persistent() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &admin, "one.txt", b"frozen parent").await;
    manager_upload(&app, &admin, "folder/two.txt", b"second parent").await;
    let listing = success(
        &app,
        &admin,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let repository = listing["context"]["repository_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let namespace = listing["context"]["branch_id"].as_str().unwrap().to_owned();
    let original = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "one.txt")
        .unwrap()
        .clone();
    let api = format!("/v1/repositories/{repository}/default-branch");
    let initial = success(&app, &admin, "GET", &api, None).await;
    let frozen_registration = success(
        &app,
        &admin,
        "POST",
        "/v1/file-manager/disks/P01/directory-branches",
        Some(json!({
            "path":"","name":"frozen","operation_id":new_id("op")
        })),
    )
    .await;
    let frozen = frozen_registration["result"]["directory_branch_id"]
        .as_str()
        .unwrap()
        .to_owned();
    manager_upload(&app, &admin, "one.txt", b"later shared branch HEAD").await;
    let later = success(&app, &admin, "GET", &api, None).await;
    assert_eq!(
        later["directory_branch_id"], initial["directory_branch_id"],
        "ordinary saves keep the named directory branch selected"
    );
    assert_eq!(later["namespace_branch_id"], namespace);

    let later_tree = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/branches/{namespace}/tree"),
        None,
    )
    .await;
    let latest_a = later_tree["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "one.txt")
        .unwrap()["version_id"]
        .clone();
    success(&app, &admin, "POST", "/v1/admin/accounts", Some(json!({
        "username":"view-writer","password":"view-writer-test-password","repositories":[repository]
    }))).await;
    success(&app, &admin, "PUT", &format!("/v1/repositories/{repository}/grants"), Some(json!({
        "grants":{"read":["view-writer"],"write":["view-writer"],"manage":["admin"]},"operation_id":new_id("op")
    }))).await;
    let writer = login(&app, "view-writer", "view-writer-test-password").await;
    assert_eq!(
        success(&app, &writer, "GET", &api, None).await["namespace_branch_id"],
        namespace
    );
    let selected = success(
        &app,
        &writer,
        "PUT",
        &api,
        Some(json!({
            "directory_branch_id":frozen,"operation_id":new_id("op")
        })),
    )
    .await;
    assert_eq!(selected["result"]["directory_branch_id"], frozen);
    let current = success(&app, &writer, "GET", &api, None).await;
    assert_eq!(current["directory_branch_id"], frozen);
    assert_eq!(current["namespace_branch_id"], namespace);
    let tree_path = format!("/v1/branches/{namespace}/tree");
    let tree = success(&app, &writer, "GET", &tree_path, None).await;
    let visible = tree["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "one.txt")
        .unwrap();
    assert_eq!(visible["version_id"], original["version_id"]);
    assert_eq!(
        version_bytes(&app, &writer, visible["version_id"].as_str().unwrap()).await,
        b"frozen parent"
    );
    let repeated = success(
        &app,
        &writer,
        "PUT",
        &api,
        Some(json!({
            "directory_branch_id":frozen,"operation_id":new_id("op")
        })),
    )
    .await;
    assert_eq!(repeated["result"]["no_change"], true);
    assert_eq!(
        success(&app, &writer, "GET", &tree_path, None).await["commit_id"],
        tree["commit_id"]
    );

    let wrong = send(
        &app,
        Some(&writer),
        "POST",
        "/v1/file-manager/disks/P01/write-sessions",
        Some(json!({
            "path":"one.txt","branch_id":namespace
        })),
    )
    .await;
    assert_eq!(wrong.0, StatusCode::CONFLICT);
    assert_eq!(wrong.1["error"]["code"], "FILE_BRANCH_CONFLICT");
    let workspace = success(
        &app,
        &writer,
        "POST",
        "/v1/file-manager/disks/P01/write-sessions",
        Some(json!({
            "path":"one.txt","branch_id":original["selected_branch_id"]
        })),
    )
    .await;
    assert_eq!(workspace["branch_id"], namespace);
    assert_eq!(workspace["base_head"], tree["commit_id"]);
    success(
        &app,
        &writer,
        "POST",
        &format!(
            "/v1/workspaces/{}/abort",
            workspace["workspace_id"].as_str().unwrap()
        ),
        Some(json!({})),
    )
    .await;
    let nested = success(
        &app,
        &writer,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=folder",
        None,
    )
    .await;
    let nested_id =
        nested["directory_branches"].as_array().unwrap()[0]["directory_branch_id"].clone();
    let wrong_scope = send(
        &app,
        Some(&writer),
        "PUT",
        &api,
        Some(json!({"directory_branch_id":nested_id,"operation_id":new_id("op")})),
    )
    .await;
    assert_eq!(wrong_scope.1["error"]["code"], "DIRECTORY_BRANCH_SCOPE");

    // Saving another file from a frozen composition must not rewind a
    // shared branch whose newer HEAD belongs to a different directory view.
    manager_upload(&app, &writer, "folder/two.txt", b"edited second file").await;
    let a_history = success(
        &app,
        &writer,
        "GET",
        &format!(
            "/v1/file-manager/files/{}/history",
            original["file_id"].as_str().unwrap()
        ),
        None,
    )
    .await;
    let shared_a = a_history["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["branch_id"] == original["selected_branch_id"])
        .unwrap();
    assert_eq!(
        shared_a["version_id"], latest_a,
        "saving a sibling must not rewind shared file branch HEAD"
    );
    success(
        &app,
        &writer,
        "PUT",
        &api,
        Some(json!({"directory_branch_id":frozen,"operation_id":new_id("op")})),
    )
    .await;

    let rename_api = format!("/v1/repositories/{repository}/directory-branches/{frozen}");
    let renamed = success(
        &app,
        &writer,
        "PATCH",
        &rename_api,
        Some(json!({
            "name":"saved view","operation_id":new_id("op")
        })),
    )
    .await;
    assert_eq!(renamed["result"]["directory_branch_id"], frozen);
    assert_eq!(
        success(&app, &writer, "GET", &api, None).await["name"],
        "saved view"
    );
    let audit = success(&app, &writer, "GET", "/v1/file-manager/history", None).await;
    for action in [
        "directory.branches.create",
        "directory.default.select",
        "directory.branch.rename",
    ] {
        assert!(
            audit["events"].as_array().unwrap().iter().any(|event| {
                event["action"] == action
                    && event["directory_branch_id"] == frozen
                    && event["file_id"] == ""
            }),
            "missing directory audit action {action}"
        );
    }
    assert!(
        audit["events"].as_array().unwrap().iter().any(|event| {
            event["action"] == "directory.tree.write"
                && event["directory_branch_id"] == frozen
                && event["head_tree_id"].is_string()
                && event["parent_tree_id"].is_string()
        }),
        "a real save appends an immutable subtree to the existing directory branch"
    );
    let overlong = send(
        &app,
        Some(&writer),
        "PATCH",
        &rename_api,
        Some(json!({
            "name":"x".repeat(129),"operation_id":new_id("op")
        })),
    )
    .await;
    assert_eq!(overlong.0, StatusCode::BAD_REQUEST);
    assert_eq!(listing["default_branch_scope"], true);
    assert_eq!(nested["default_branch_scope"], false);

    success(&app, &admin, "PUT", &format!("/v1/repositories/{repository}/grants"), Some(json!({
        "grants":{"read":["view-writer"],"write":[],"manage":["admin"]},"operation_id":new_id("op")
    }))).await;
    assert_eq!(
        send(&app, Some(&writer), "GET", &api, None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            Some(&writer),
            "PUT",
            &api,
            Some(json!({"directory_branch_id":frozen,"operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let empty = success(&app, &admin, "POST", "/v1/repositories", Some(json!({
        "name":"Empty view","primary_slot_id":"P01","target_copies":3,"operation_id":new_id("op")
    }))).await;
    let empty_id = empty["repository_id"].as_str().unwrap();
    let empty_default = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/repositories/{empty_id}/default-branch"),
        None,
    )
    .await;
    assert!(empty_default["directory_branch_id"].is_null());
    assert!(
        empty_default["directory_branches"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    capture_wire("default-directory-branch.json", &current);
    drop(app);
    let recovered = reopen_after_router_drop(&root.path().join("system")).await;
    assert_eq!(
        recovered.state().repositories[&repository].main_branch_id,
        namespace
    );
    assert_eq!(recovered.state().directory_selections[&repository], frozen);
    assert_eq!(
        recovered.state().directory_branches[&frozen].name,
        "saved view"
    );
    let visible = recovered
        .list_tree(&namespace)
        .unwrap()
        .into_iter()
        .find(|e| e.name == "one.txt")
        .unwrap();
    assert_eq!(visible.version_id, original["version_id"]);
}

async fn manager_directory_branch(app: &Router, token: &str, path: &str, name: &str) -> String {
    success(
        app,
        token,
        "POST",
        "/v1/file-manager/disks/P01/directory-branches",
        Some(json!({
            "path":path,"name":name,"operation_id":new_id("op")
        })),
    )
    .await["result"]["directory_branch_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn manager_scope(app: &Router, token: &str, repo: &str, branch: &str) -> Value {
    success(
        app,
        token,
        "GET",
        &format!("/v1/repositories/{repo}/directory-branches"),
        None,
    )
    .await["directory_branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scope| scope["directory_branch_id"] == branch)
        .unwrap()
        .clone()
}

fn manager_batch_source(scope: &Value, path: &str) -> Value {
    let entry = scope["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == path)
        .unwrap();
    json!({"directory_branch_id":scope["directory_branch_id"],"expected_tree_id":scope["head_tree_id"],"path":path,"entry_id":entry["entry_id"]})
}
fn manager_batch_target(scope: &Value, path: &str) -> Value {
    json!({"directory_branch_id":scope["directory_branch_id"],"expected_tree_id":scope["head_tree_id"],"path":path})
}

#[tokio::test]
async fn branch_namespace_batch_preserves_unselected_views_and_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "a/report.txt", b"frozen source").await;
    manager_upload(&app, &token, "b/stay.txt", b"target sibling").await;
    let root_view = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let repo = root_view["context"]["repository_id"].as_str().unwrap();
    let a_m = manager_directory_branch(&app, &token, "a", "m").await;
    let a_n = manager_directory_branch(&app, &token, "a", "n").await;
    let a_r = manager_directory_branch(&app, &token, "a", "r").await;
    let b_l = manager_directory_branch(&app, &token, "b", "l").await;
    let b_p = manager_directory_branch(&app, &token, "b", "p").await;
    let b_q = manager_directory_branch(&app, &token, "b", "q").await;
    let m = manager_scope(&app, &token, repo, &a_m).await;
    let n = manager_scope(&app, &token, repo, &a_n).await;
    let r = manager_scope(&app, &token, repo, &a_r).await;
    let l = manager_scope(&app, &token, repo, &b_l).await;
    let p = manager_scope(&app, &token, repo, &b_p).await;
    let q = manager_scope(&app, &token, repo, &b_q).await;
    let original = m["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["path"] == "a/report.txt")
        .unwrap()
        .clone();
    let default_before = success(
        &app,
        &token,
        "GET",
        &format!("/v1/repositories/{repo}/default-branch"),
        None,
    )
    .await;
    let request = json!({"kind":"move","operation_id":new_id("op"),"items":[
        {"source":manager_batch_source(&m,"a/report.txt"),"target":manager_batch_target(&l,"b/report.txt")},
        {"source":manager_batch_source(&n,"a/report.txt"),"target":manager_batch_target(&p,"b/report.txt")}
    ]});
    let route = "/v1/file-manager/disks/P01/namespace-batch";
    let moved = success(&app, &token, "POST", route, Some(request.clone())).await;
    let repeated = success(&app, &token, "POST", route, Some(request.clone())).await;
    assert_eq!(repeated["operation_id"], moved["operation_id"]);
    assert_eq!(
        repeated["result"], moved["result"],
        "retry does not move or append trees twice"
    );
    for id in [&a_m, &a_n] {
        let scope = manager_scope(&app, &token, repo, id).await;
        assert!(
            scope["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["file_id"] != original["file_id"])
        );
    }
    for id in [&b_l, &b_p] {
        let scope = manager_scope(&app, &token, repo, id).await;
        let moved = scope["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["path"] == "b/report.txt")
            .unwrap();
        assert_eq!(moved["file_id"], original["file_id"]);
        assert_eq!(moved["version_id"], original["version_id"]);
    }
    assert_eq!(
        manager_scope(&app, &token, repo, &a_r).await["head_tree_id"],
        r["head_tree_id"]
    );
    assert_eq!(
        manager_scope(&app, &token, repo, &b_q).await["head_tree_id"],
        q["head_tree_id"]
    );
    assert_eq!(
        version_bytes(&app, &token, original["version_id"].as_str().unwrap()).await,
        b"frozen source"
    );
    assert_eq!(
        success(
            &app,
            &token,
            "GET",
            &format!("/v1/repositories/{repo}/default-branch"),
            None
        )
        .await["directory_branch_id"],
        default_before["directory_branch_id"],
        "batch editing inactive branches does not change SMB default"
    );

    let l = manager_scope(&app, &token, repo, &b_l).await;
    let p = manager_scope(&app, &token, repo, &b_p).await;
    let copied=success(&app,&token,"POST",route,Some(json!({"kind":"copy","operation_id":new_id("op"),"items":[
        {"source":manager_batch_source(&l,"b/report.txt"),"target":manager_batch_target(&l,"b/copy.txt")},
        {"source":manager_batch_source(&p,"b/report.txt"),"target":manager_batch_target(&p,"b/copy.txt")}
    ]}))).await;
    assert_eq!(copied["state"], "completed");
    let l = manager_scope(&app, &token, repo, &b_l).await;
    let p = manager_scope(&app, &token, repo, &b_p).await;
    let copy_l = l["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["path"] == "b/copy.txt")
        .unwrap();
    let copy_p = p["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["path"] == "b/copy.txt")
        .unwrap();
    assert_ne!(copy_l["file_id"], original["file_id"]);
    assert_eq!(
        copy_l["file_id"], copy_p["file_id"],
        "one copy identity shared by selected target scopes"
    );
    assert_ne!(copy_l["version_id"], original["version_id"]);
    assert_eq!(
        version_bytes(&app, &token, copy_l["version_id"].as_str().unwrap()).await,
        b"frozen source"
    );
    let copy_version = copy_l["version_id"].clone();

    success(&app,&token,"POST",route,Some(json!({"kind":"rename","operation_id":new_id("op"),"items":[
        {"source":manager_batch_source(&l,"b/copy.txt"),"target":manager_batch_target(&l,"b/客户报告.txt")},
        {"source":manager_batch_source(&p,"b/copy.txt"),"target":manager_batch_target(&p,"b/客户报告.txt")}
    ]}))).await;
    let l = manager_scope(&app, &token, repo, &b_l).await;
    let p = manager_scope(&app, &token, repo, &b_p).await;
    assert!(
        l["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["path"] == "b/客户报告.txt" && e["version_id"] == copy_version)
    );
    // Deleting one branch keeps the other branch's same immutable copy readable.
    success(
        &app,
        &token,
        "POST",
        route,
        Some(json!({"kind":"delete","operation_id":new_id("op"),"items":[
            {"source":manager_batch_source(&l,"b/客户报告.txt")}
        ]})),
    )
    .await;
    assert!(
        manager_scope(&app, &token, repo, &b_l).await["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["path"] != "b/客户报告.txt")
    );
    assert_eq!(
        manager_scope(&app, &token, repo, &b_p).await["head_tree_id"],
        p["head_tree_id"]
    );
    assert_eq!(
        version_bytes(&app, &token, copy_version.as_str().unwrap()).await,
        b"frozen source"
    );

    // One stale mapping prevents the entire batch, including the otherwise valid first row.
    let p = manager_scope(&app, &token, repo, &b_p).await;
    let l = manager_scope(&app, &token, repo, &b_l).await;
    let conflict = send(
        &app,
        Some(&token),
        "POST",
        route,
        Some(json!({"kind":"delete","operation_id":new_id("op"),"items":[
            {"source":manager_batch_source(&p,"b/客户报告.txt")},
            {"source":manager_batch_source(&m,"a/report.txt")}
        ]})),
    )
    .await;
    assert_eq!(conflict.0, StatusCode::CONFLICT);
    assert_eq!(
        manager_scope(&app, &token, repo, &b_p).await["head_tree_id"],
        p["head_tree_id"]
    );
    assert_eq!(
        manager_scope(&app, &token, repo, &b_l).await["head_tree_id"],
        l["head_tree_id"]
    );
    assert_eq!(
        success(
            &app,
            &token,
            "GET",
            &format!("/v1/repositories/{repo}/directory-branches?path=b"),
            None
        )
        .await["directory_branches"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let readonly = send(
        &app,
        Some(&token),
        "POST",
        "/v1/file-manager/disks/B01/namespace-batch",
        Some(request),
    )
    .await;
    assert_eq!(readonly.0, StatusCode::FORBIDDEN);
    success(&app,&token,"POST","/v1/admin/accounts",Some(json!({
        "username":"branch-reader","password":"branch-reader-test-password","repositories":[repo]
    }))).await;
    success(&app,&token,"PUT",&format!("/v1/repositories/{repo}/grants"),Some(json!({
        "grants":{"read":["branch-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")
    }))).await;
    let reader = login(&app, "branch-reader", "branch-reader-test-password").await;
    let visible = manager_scope(&app, &reader, repo, &b_p).await;
    let denied = send(
        &app,
        Some(&reader),
        "POST",
        route,
        Some(json!({"kind":"delete","operation_id":new_id("op"),"items":[
            {"source":manager_batch_source(&visible,"b/客户报告.txt")}
        ]})),
    )
    .await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert_eq!(
        manager_scope(&app, &token, repo, &b_p).await["head_tree_id"],
        visible["head_tree_id"]
    );
    let audit = success(&app, &token, "GET", "/v1/file-manager/history", None).await;
    for action in [
        "directory.namespace.move",
        "directory.namespace.copy",
        "directory.namespace.rename",
        "directory.namespace.delete",
    ] {
        assert!(
            audit["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["action"] == action && event["directory_branch_id"].is_string()),
            "missing branch-scoped audit {action}"
        );
    }
    capture_wire("branch-namespace-batch.json", &moved);
}

#[tokio::test]
async fn moved_file_aliases_keep_independent_path_contexts_for_history_and_write() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "a/report.txt", b"original").await;
    manager_upload(&app, &token, "b/stay.txt", b"sibling").await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=a",
        None,
    )
    .await;
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let original = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "report.txt")
        .unwrap()
        .clone();
    assert_eq!(original["repository_path"], "a/report.txt");
    let m = manager_directory_branch(&app, &token, "a", "m").await;
    let n = manager_directory_branch(&app, &token, "a", "n").await;
    let l = manager_directory_branch(&app, &token, "b", "l").await;
    let m0 = manager_scope(&app, &token, repo, &m).await;
    let l0 = manager_scope(&app, &token, repo, &l).await;
    success(&app,&token,"POST","/v1/file-manager/disks/P01/namespace-batch",Some(json!({"kind":"move","operation_id":new_id("op"),"items":[{
        "source":manager_batch_source(&m0,"a/report.txt"),"target":manager_batch_target(&l0,"b/report.txt")
    }]}))).await;
    for (path, name) in [("a", "n"), ("b", "l")] {
        success(
            &app,
            &token,
            "POST",
            "/v1/file-manager/disks/P01/directory-select",
            Some(json!({"path":path,"name":name,"operation_id":new_id("op")})),
        )
        .await;
    }
    let a_before = manager_scope(&app, &token, repo, &n).await;
    let file = original["file_id"].as_str().unwrap();
    let history_route = |path: &str| {
        format!("/v1/file-manager/files/{file}/history?repository_id={repo}&path={path}")
    };
    assert_eq!(
        success(&app, &token, "GET", &history_route("a/report.txt"), None).await["version_id"],
        original["version_id"]
    );
    manager_upload(&app, &token, "b/report.txt", b"written in b").await;
    let a = success(&app, &token, "GET", &history_route("a/report.txt"), None).await;
    let b = success(&app, &token, "GET", &history_route("b/report.txt"), None).await;
    assert_eq!(a["version_id"], original["version_id"]);
    assert_ne!(b["version_id"], original["version_id"]);
    assert_ne!(a["selected_branch_id"], b["selected_branch_id"]);
    assert_eq!(a["inherited"], true);
    assert_eq!(b["inherited"], false);
    let index = root.path().join("system/index.sqlite");
    let saved_index = root.path().join("system/index.saved");
    std::fs::rename(&index, &saved_index).unwrap();
    std::fs::create_dir(&index).unwrap();
    for (path, expected) in [("a/report.txt", &a), ("b/report.txt", &b)] {
        assert_eq!(
            &success(&app, &token, "GET", &history_route(path), None).await,
            expected,
            "the small-store scan must retain alias selection with an unavailable cache"
        );
    }
    std::fs::remove_dir(&index).unwrap();
    std::fs::rename(&saved_index, &index).unwrap();
    assert_eq!(
        manager_scope(&app, &token, repo, &n).await["head_tree_id"],
        a_before["head_tree_id"],
        "b write must not alter the retained a alias"
    );
    assert_eq!(
        version_bytes(&app, &token, a["version_id"].as_str().unwrap()).await,
        b"original"
    );
    assert_eq!(
        version_bytes(&app, &token, b["version_id"].as_str().unwrap()).await,
        b"written in b"
    );
    success(&app,&token,"POST",&format!("/v1/file-manager/files/{file}/select"),Some(json!({
        "repository_id":repo,"path":"a/report.txt","branch_id":b["selected_branch_id"],"operation_id":new_id("op")
    }))).await;
    assert_eq!(
        success(&app, &token, "GET", &history_route("a/report.txt"), None).await["version_id"],
        b["version_id"]
    );
    assert_eq!(
        success(&app, &token, "GET", &history_route("b/report.txt"), None).await["version_id"],
        b["version_id"]
    );
    assert_eq!(
        send(
            &app,
            Some(&token),
            "GET",
            &history_route("missing.txt"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let a_scope = manager_scope(&app, &token, repo, &n).await;
    success(&app,&token,"POST","/v1/file-manager/disks/P01/namespace-batch",Some(json!({"kind":"rename","operation_id":new_id("op"),"items":[{
        "source":manager_batch_source(&a_scope,"a/report.txt"),"target":manager_batch_target(&a_scope,"a/archive.txt")
    }]}))).await;
    for (path, name) in [
        ("a/archive.txt", "archive.txt"),
        ("b/report.txt", "report.txt"),
    ] {
        let uri = format!(
            "/v1/versions/{}/content?repository_id={repo}&path={path}",
            original["version_id"].as_str().unwrap()
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()["content-disposition"]
                .to_str()
                .unwrap()
                .contains(&format!("filename=\"{name}\""))
        );
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 64)
                .await
                .unwrap()
                .as_ref(),
            b"original"
        );
    }
    let mismatched = format!(
        "/v1/versions/{}/content?repository_id={repo}&path=b/stay.txt",
        original["version_id"].as_str().unwrap()
    );
    assert_eq!(
        send(&app, Some(&token), "GET", &mismatched, None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn public_file_branch_label_patch_preserves_content_and_directory_nodes() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &admin, "report.txt", b"unchanged bytes").await;
    let listing = success(
        &app,
        &admin,
        "GET",
        "/v1/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let file = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "report.txt")
        .unwrap();
    let id = file["file_id"].as_str().unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let created=success(&app,&admin,"POST",&format!("/v1/file-manager/files/{id}/branches"),Some(json!({"source_version_id":file["version_id"],"name":"draft","operation_id":new_id("op")}))).await;
    let branch = created["result"]["branch_id"].as_str().unwrap();
    let route = format!("/v1/file-manager/files/{id}/branches/{branch}");
    let scopes = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/repositories/{repo}/directory-branches"),
        None,
    )
    .await;
    let before = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/file-manager/files/{id}/history"),
        None,
    )
    .await;
    let body = json!({"name":"客户主稿","operation_id":new_id("op")});
    let result = success(&app, &admin, "PATCH", &route, Some(body.clone())).await;
    assert_eq!(result["result"]["name"], "客户主稿");
    assert_eq!(
        success(&app, &admin, "PATCH", &route, Some(body)).await["result"],
        result["result"]
    );
    let after = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/file-manager/files/{id}/history"),
        None,
    )
    .await;
    assert_eq!(after["versions"], before["versions"]);
    assert_eq!(after["selected_branch_id"], before["selected_branch_id"]);
    assert!(
        after["branches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["branch_id"] == branch && item["name"] == "客户主稿")
    );
    assert_eq!(
        success(
            &app,
            &admin,
            "GET",
            &format!("/v1/repositories/{repo}/directory-branches"),
            None
        )
        .await,
        scopes
    );
    assert_eq!(
        version_bytes(&app, &admin, file["version_id"].as_str().unwrap()).await,
        b"unchanged bytes"
    );
    let collision = send(
        &app,
        Some(&admin),
        "PATCH",
        &route,
        Some(json!({"name":"MAIN","operation_id":new_id("op")})),
    )
    .await;
    assert_eq!(collision.0, StatusCode::CONFLICT);
    assert_eq!(collision.1["error"]["code"], "NAME_COLLISION");
    assert_eq!(
        send(
            &app,
            Some(&admin),
            "PATCH",
            &route,
            Some(json!({"name":"文".repeat(43),"operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    success(&app,&admin,"POST","/v1/admin/accounts",Some(json!({"username":"label-reader","password":"label-reader-test-password","repositories":[repo]}))).await;
    success(&app,&admin,"PUT",&format!("/v1/repositories/{repo}/grants"),Some(json!({"grants":{"read":["label-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "label-reader", "label-reader-test-password").await;
    assert_eq!(
        send(
            &app,
            Some(&reader),
            "PATCH",
            &route,
            Some(json!({"name":"denied","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let audit = success(&app, &admin, "GET", "/v1/file-manager/history", None).await;
    assert!(
        audit["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["action"] == "file.branch.rename"
                && event["branch_id"] == branch
                && event["message"] == "客户主稿")
    );
}

#[tokio::test]
async fn rename_nodes_reuse_bodies_and_preserve_graph_parents_notes_and_branch_sources() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &admin, "documents/one.txt", b"original one").await;
    manager_upload(&app, &admin, "documents/nested/two.txt", b"original two").await;
    let listing = success(
        &app,
        &admin,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=documents",
        None,
    )
    .await;
    let one = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "one.txt")
        .unwrap();
    let file = one["file_id"].as_str().unwrap();
    let body = one["version_id"].as_str().unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let history_route = format!("/v1/file-manager/files/{file}/history");
    let before = success(&app, &admin, "GET", &history_route, None).await;
    assert_eq!(before["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(before["node_id"], body);
    success(&app,&admin,"POST","/v1/file-manager/disks/P01/namespace",Some(json!({"kind":"rename","path":"documents","destination":"archive","operation_id":new_id("op")}))).await;
    let after = success(
        &app,
        &admin,
        "GET",
        &format!("{history_route}?repository_id={repo}&path=archive/one.txt"),
        None,
    )
    .await;
    assert_eq!(
        after["versions"], before["versions"],
        "renaming a folder reuses the real content DTO and body"
    );
    assert_eq!(after["nodes"].as_array().unwrap().len(), 2);
    let path_node = after["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["kind"] == "rename")
        .unwrap()
        .clone();
    let node = path_node["node_id"].as_str().unwrap();
    assert!(node.starts_with("fnode_"));
    assert_eq!(path_node["version_id"], body);
    assert_eq!(path_node["parent_node_ids"], json!([body]));
    assert_eq!(path_node["path"], "archive/one.txt");
    assert_eq!(path_node["previous_path"], "documents/one.txt");
    assert_eq!(path_node["current"], true);
    assert_eq!(after["node_id"], node);
    assert_eq!(after["current_node_id"], node);
    let nested = success(
        &app,
        &admin,
        "GET",
        "/v1/file-manager/disks/P01/entries?path=archive/nested",
        None,
    )
    .await;
    let two = &nested["entries"][0];
    let two_file = two["file_id"].as_str().unwrap();
    let two_node = two["node_id"].as_str().unwrap();
    assert!(two_node.starts_with("fnode_"));
    assert_ne!(two_node, node);
    let two_history = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/file-manager/files/{two_file}/history"),
        None,
    )
    .await;
    assert_eq!(two_history["versions"].as_array().unwrap().len(), 1);
    assert_eq!(two_history["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(version_bytes(&app, &admin, body).await, b"original one");
    let scopes = success(
        &app,
        &admin,
        "GET",
        &format!("/v1/repositories/{repo}/directory-branches"),
        None,
    )
    .await;
    let note_route = format!("/v1/file-manager/files/{file}/nodes/{node}");
    success(
        &app,
        &admin,
        "PATCH",
        &note_route,
        Some(json!({"note":"路径改名备注","operation_id":new_id("op")})),
    )
    .await;
    success(
        &app,
        &admin,
        "POST",
        &format!("/v1/file-manager/versions/{body}/metadata"),
        Some(json!({"note":"正文备注","operation_id":new_id("op")})),
    )
    .await;
    let noted = success(&app, &admin, "GET", &history_route, None).await;
    assert_eq!(
        noted["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["node_id"] == node)
            .unwrap()["note"],
        "路径改名备注"
    );
    assert_eq!(
        noted["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["node_id"] == body)
            .unwrap()["note"],
        "正文备注"
    );
    assert_eq!(
        success(
            &app,
            &admin,
            "GET",
            &format!("/v1/repositories/{repo}/directory-branches"),
            None
        )
        .await,
        scopes,
        "notes add no directory subtree"
    );
    for invalid in ["x".repeat(129), "文".repeat(43), "bad\nline".into()] {
        assert_eq!(
            send(
                &app,
                Some(&admin),
                "PATCH",
                &note_route,
                Some(json!({"note":invalid,"operation_id":new_id("op")}))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let wrong_note = format!("/v1/file-manager/files/{file}/nodes/{two_node}");
    assert_eq!(
        send(
            &app,
            Some(&admin),
            "PATCH",
            &wrong_note,
            Some(json!({"note":"wrong file","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let branch_route = format!("/v1/file-manager/files/{file}/branches");
    assert_eq!(send(&app,Some(&admin),"POST",&branch_route,Some(json!({"source_version_id":body,"source_node_id":two_node,"name":"foreign","operation_id":new_id("op")}))).await.0,StatusCode::NOT_FOUND);
    let branch=success(&app,&admin,"POST",&branch_route,Some(json!({"source_version_id":body,"source_node_id":node,"name":"from-path","operation_id":new_id("op")}))).await;
    let branch_id = branch["result"]["branch_id"].as_str().unwrap();
    let registered = success(&app, &admin, "GET", &history_route, None).await;
    assert_eq!(registered["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(registered["versions"].as_array().unwrap().len(), 1);
    assert!(
        registered["branches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|branch| branch["branch_id"] == branch_id && branch["file_node_id"] == node)
    );
    success(&app,&admin,"POST",&format!("/v1/file-manager/files/{file}/select"),Some(json!({"repository_id":repo,"path":"archive/one.txt","branch_id":branch_id,"operation_id":new_id("op")}))).await;
    assert_eq!(
        success(&app, &admin, "GET", &history_route, None).await["node_id"],
        node
    );
    manager_upload(&app, &admin, "archive/one.txt", b"new body after path node").await;
    let written = success(&app, &admin, "GET", &history_route, None).await;
    assert_eq!(written["versions"].as_array().unwrap().len(), 2);
    assert_eq!(written["nodes"].as_array().unwrap().len(), 3);
    let current_body = written["version_id"].as_str().unwrap();
    assert_ne!(current_body, body);
    assert_eq!(written["node_id"], current_body);
    let content = written["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_id"] == current_body)
        .unwrap();
    assert_eq!(content["kind"], "content");
    assert_eq!(content["parent_node_ids"], json!([node]));
    assert_eq!(
        content["source_version_ids"],
        json!([body]),
        "real body provenance remains separate from the file-node graph"
    );
    assert_eq!(content["current"], true);
    assert_eq!(written["nodes"][0]["node_id"], current_body);
    assert_eq!(version_bytes(&app, &admin, body).await, b"original one");
    assert_eq!(
        version_bytes(&app, &admin, current_body).await,
        b"new body after path node"
    );
    success(&app,&admin,"POST",&branch_route,Some(json!({"source_version_id":current_body,"source_node_id":current_body,"name":"content-source","operation_id":new_id("op")}))).await;
    assert_eq!(send(&app,Some(&admin),"POST",&branch_route,Some(json!({"source_version_id":current_body,"source_node_id":node,"name":"mismatched-body","operation_id":new_id("op")}))).await.0,StatusCode::NOT_FOUND);
    success(&app,&admin,"POST","/v1/admin/accounts",Some(json!({"username":"path-reader","password":"path-reader-test-password","repositories":[repo]}))).await;
    success(&app,&admin,"PUT",&format!("/v1/repositories/{repo}/grants"),Some(json!({"grants":{"read":["path-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "path-reader", "path-reader-test-password").await;
    assert_eq!(
        send(
            &app,
            Some(&reader),
            "PATCH",
            &note_route,
            Some(json!({"note":"denied","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        success(&app, &reader, "GET", &history_route, None).await["nodes"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let audit = success(&app, &admin, "GET", "/v1/file-manager/history", None).await;
    for action in ["file.path.rename", "file.path.metadata"] {
        assert!(
            audit["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["action"] == action && event["node_id"] == node)
        );
    }
    capture_wire("file-path-history.json", &written);
}

#[tokio::test]
async fn engine_upload_generation_conflicts_reach_http_as_conflicts() {
    let root = tempfile::tempdir().unwrap();
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let backend = Arc::new(EngineBackend::new(
        Engine::initialize(&root.path().join("system"), config).unwrap(),
    ));
    let auth = backend
        .protect_auth(
            AuthStore::initialize(
                &root.path().join("system"),
                "admin",
                "integration-test-password",
            )
            .unwrap(),
        )
        .unwrap();
    let app = zxs_server::router(
        backend.clone(),
        auth,
        ServerConfig::loopback(&root.path().join("system")),
    )
    .unwrap();
    let token = login(&app, "admin", "integration-test-password").await;
    let repo = success(
        &app,
        &token,
        "POST",
        "/v1/repositories",
        Some(json!({
            "name":"Cross-entry uploads", "primary_slot_id":"P01", "target_copies":1,
            "operation_id":new_id("op")
        })),
    )
    .await;
    let workspace = success(
        &app,
        &token,
        "POST",
        "/v1/workspaces",
        Some(json!({
            "branch_id":repo["main_branch_id"]
        })),
    )
    .await;
    let ws = workspace["workspace_id"].as_str().unwrap();
    let item = new_id("entry");
    let source = root.path().join("source");
    std::fs::write(&source, b"original").unwrap();
    // A different adapter can stage directly in the engine without HTTP spool
    // metadata. The engine's conflict must keep the same public HTTP semantics.
    backend
        .upload_generation(
            &Identity {
                principal: "admin".into(),
                admin: true,
                repositories: Default::default(),
            },
            ws,
            &item,
            "file.txt",
            &source,
            2,
        )
        .unwrap();
    for generation in [1, 2] {
        let bytes = b"different";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/v1/workspaces/{ws}/files/{item}/content?path=file.txt"
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-zxs-logical-size", bytes.len().to_string())
                    .header("x-zxs-content-hash", blake3::hash(bytes).to_hex().as_str())
                    .header("x-zxs-upload-generation", generation.to_string())
                    .body(Body::from(bytes.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "generation {generation}"
        );
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["error"]["code"], "UPLOAD_GENERATION_CONFLICT");
    }
}

#[tokio::test]
async fn interrupted_new_upload_generation_reports_its_own_durable_progress() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    let repo = success(
        &app,
        &token,
        "POST",
        "/v1/repositories",
        Some(json!({
            "name":"Upload generations", "primary_slot_id":"P01", "target_copies":3,
            "operation_id":new_id("op")
        })),
    )
    .await;
    let workspace = success(
        &app,
        &token,
        "POST",
        "/v1/workspaces",
        Some(json!({
            "branch_id":repo["main_branch_id"]
        })),
    )
    .await;
    let ws = workspace["workspace_id"].as_str().unwrap();
    let item = new_id("entry");
    upload(&app, &token, ws, &item, "file.txt", b"original").await;
    let bytes = b"replacement bytes";
    for complete in [false, true] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/v1/workspaces/{ws}/files/{item}/content?path=file.txt"
                    ))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-zxs-logical-size", bytes.len().to_string())
                    .header("x-zxs-content-hash", blake3::hash(bytes).to_hex().as_str())
                    .header("x-zxs-upload-generation", "2")
                    .body(Body::from(if complete {
                        bytes.to_vec()
                    } else {
                        bytes[..5].to_vec()
                    }))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            if complete {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            }
        );
        let status = success(
            &app,
            &token,
            "GET",
            &format!("/v1/workspaces/{ws}/files/{item}/upload-status"),
            None,
        )
        .await;
        assert_eq!(status["upload_generation"], "2");
        assert_eq!(
            status["state"],
            if complete { "COMPLETE" } else { "RECEIVING" }
        );
        assert_eq!(status["logical_size"], bytes.len().to_string());
        assert_eq!(
            status["content_hash"],
            blake3::hash(bytes).to_hex().as_str()
        );
        assert_eq!(
            status["durable_ranges"][0]["length"],
            if complete {
                bytes.len().to_string()
            } else {
                "5".into()
            }
        );
    }
    success(
        &app,
        &token,
        "POST",
        &format!("/v1/workspaces/{ws}/seal"),
        Some(json!({})),
    )
    .await;
    let status = success(
        &app,
        &token,
        "GET",
        &format!("/v1/workspaces/{ws}/files/{item}/upload-status"),
        None,
    )
    .await;
    assert_eq!(status["upload_generation"], "2");
    assert_eq!(status["state"], "SEALED");
    assert!(status["sealed_manifest_hash"].is_string());
}

#[tokio::test]
async fn directory_scope_initialization_accepts_root_but_requires_a_string_path() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    let repo = success(
        &app,
        &token,
        "POST",
        "/v1/repositories",
        Some(json!({
            "name":"Root scope", "primary_slot_id":"P01", "target_copies":3,
            "operation_id":new_id("op")
        })),
    )
    .await;
    let endpoint = format!(
        "/v1/repositories/{}/directory-scopes",
        repo["repository_id"].as_str().unwrap()
    );
    for path in [
        None,
        Some(Value::Null),
        Some(json!(12)),
        Some(json!("../outside")),
    ] {
        let mut body = json!({"operation_id":new_id("op")});
        if let Some(path) = path {
            body["path"] = path;
        }
        assert_eq!(
            send(&app, Some(&token), "POST", &endpoint, Some(body))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let initialized = success(
        &app,
        &token,
        "POST",
        &endpoint,
        Some(json!({"path":"", "operation_id":new_id("op")})),
    )
    .await;
    assert_eq!(initialized["state"], "completed");
    assert!(initialized["result"]["directory_branch_id"].is_string());
}
