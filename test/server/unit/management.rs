use crate::Backend;
use crate::{ServerConfig, auth::AuthStore, management::*, router};
use http_routing::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use json_codec::{Value, json};
use service_stack::ServiceExt;
async fn request(
    app: &Router,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut r = Request::builder()
        .uri(path)
        .method(if body.is_some() { "POST" } else { "GET" })
        .header("content-type", "application/json");
    if let Some(token) = token {
        r = r.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(
            r.body(
                body.map(|b| Body::from(json_codec::to_vec(&b).unwrap()))
                    .unwrap_or_else(Body::empty),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = http_routing::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, json_codec::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn missing_data_disks_leave_login_diagnostics_and_health_usable() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = root.path().join("management");
    let disks = [
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ];
    drop(
        storage_engine::Engine::initialize(
            &data,
            storage_contract::Config::development(disks.clone()),
        )
        .unwrap(),
    );
    for disk in disks {
        std::fs::remove_dir_all(disk).unwrap();
    }
    drop(AuthStore::initialize_management(&management, "admin", "a-long-test-password").unwrap());
    let (backend, auth) = open_service(&data, &management).unwrap();
    let app = router(backend.clone(), auth, ServerConfig::loopback(&management)).unwrap();
    assert_eq!(
        request(&app, "/healthz", None, None).await.0,
        StatusCode::OK
    );
    let (status, result) = request(
        &app,
        "/api/auth/login",
        None,
        Some(json!({"username":"admin","password":"a-long-test-password"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = result["token"].as_str().unwrap();
    assert_eq!(
        request(&app, "/api/auth/me", Some(token), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/system/storage", Some(token), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/system/status", Some(token), None)
            .await
            .0,
        StatusCode::OK
    );
    let (status, error) = request(&app, "/api/repositories", Some(token), Some(json!({}))).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error["error"]["code"], "STORAGE_UNAVAILABLE");
    assert_eq!(
        request(&app, "/api/system/storage/retry", None, Some(json!({})))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for _ in 0..200 {
        if !backend.management_status()["recovering"].as_bool().unwrap() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(backend.management_status()["state"], "unavailable");
}
#[tokio::test]
async fn stale_legacy_credentials_do_not_open_management_when_engine_fails() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = root.path().join("management");
    drop(AuthStore::initialize(&data, "admin", "a-long-test-password").unwrap());
    std::fs::write(data.join("config.json"), "corrupt").unwrap();
    let (backend, auth) = open_service(&data, &management).unwrap();
    let app = router(backend, auth, ServerConfig::loopback(&management)).unwrap();
    assert_eq!(
        request(&app, "/healthz", None, None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/management/status", None, None).await.1["authentication"],
        "recovery_required"
    );
    assert_eq!(
        request(
            &app,
            "/api/auth/login",
            None,
            Some(json!({"username":"admin","password":"a-long-test-password"}))
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(!management.join("management-auth.json").exists());
}
#[test]
fn replica_failure_cannot_roll_back_revocation_or_password_change() {
    let root = tempfile::tempdir().unwrap();
    let mut auth =
        AuthStore::initialize_management(root.path(), "admin", "a-long-test-password").unwrap();
    let old = auth.login("admin", "a-long-test-password").unwrap();
    auth.attach_replica(std::sync::Arc::new(|_| {
        Err(crate::ApiError::new(503, "DISK_OFFLINE", "disk offline"))
    }));
    auth.change_password(&old.token, "a-long-test-password", "a-new-test-password")
        .unwrap();
    assert!(auth.authenticate(&old.token).is_err());
    drop(auth);
    let mut auth = AuthStore::open_management(root.path()).unwrap();
    assert!(auth.authenticate(&old.token).is_err());
    assert!(auth.login("admin", "a-long-test-password").is_err());
    assert!(auth.login("admin", "a-new-test-password").is_ok());
}

#[tokio::test]
async fn blocked_replication_never_blocks_login_and_queued_sessions_revalidate() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = root.path().join("management");
    drop(
        storage_engine::Engine::initialize(
            &data,
            storage_contract::Config::development([
                root.path().join("d1"),
                root.path().join("d2"),
                root.path().join("d3"),
            ]),
        )
        .unwrap(),
    );
    let mut auth =
        AuthStore::initialize_management(&management, "admin", "a-long-test-password").unwrap();
    auth.upsert_account(
        "reader",
        Some("a-reader-test-password"),
        false,
        Default::default(),
        false,
    )
    .unwrap();
    drop(auth);
    let (backend, auth) = open_service(&data, &management).unwrap();
    let app = router(backend.clone(), auth, ServerConfig::loopback(&management)).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if backend.management_status()["state"] == "ready" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let shared = backend.inner.auth.lock().unwrap().clone().unwrap();
    let session = shared
        .lock()
        .unwrap()
        .login("reader", "a-reader-test-password")
        .unwrap();
    let stale = shared.lock().unwrap().authenticate(&session.token).unwrap();
    let engine = backend.inner.state.lock().unwrap().backend.clone().unwrap();
    let (held_tx, held_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let held_engine = engine.clone();
    let hold = std::thread::spawn(move || {
        held_engine.test_with_engine_locked(|| {
            held_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        })
    });
    held_rx.recv().unwrap();
    let queued = std::thread::spawn(move || {
        engine.call(crate::BackendAction::Repositories, &stale, json!({}))
    });
    shared
        .lock()
        .unwrap()
        .upsert_account("reader", None, false, Default::default(), true)
        .unwrap();
    // This is a deadlock guard, not a latency benchmark. The engine remains
    // locked until the result is captured, even on slower test runners.
    let login = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        request(
            &app,
            "/api/auth/login",
            None,
            Some(json!({"username":"admin","password":"a-long-test-password"})),
        ),
    )
    .await;
    // Always release the deliberately blocked worker before asserting.
    release_tx.send(()).unwrap();
    hold.join().unwrap();
    assert_eq!(login.unwrap().0, StatusCode::OK);
    assert_eq!(queued.join().unwrap().unwrap_err().code, "UNAUTHORIZED");
    assert_eq!(
        request(&app, "/api/auth/me", Some(&session.token), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn protected_web_setup_installs_primary_then_allows_login_without_restart() {
    exercise_web_setup(&["one", "two", "three"], false).await;
}

#[tokio::test]
async fn single_disk_web_setup_ready_then_missing_disk_preserves_management() {
    exercise_web_setup(&["one"], true).await;
}

async fn exercise_web_setup(ids: &[&str], remove_data_disk: bool) {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = data.join("management");
    let mut disks = std::collections::BTreeMap::new();
    for &id in ids {
        let path = root.path().join(id);
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        disks.insert(id.to_owned(), path);
    }
    let (backend, auth) = open_service_with_disks(&data, &management, disks, true).unwrap();
    let key = "a".repeat(64);
    std::fs::write(management.join("bootstrap.key"), &key).unwrap();
    std::fs::set_permissions(
        management.join("bootstrap.key"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let app = router(backend.clone(), auth, ServerConfig::loopback(&management)).unwrap();
    assert_eq!(
        request(&app, "/api/setup/status", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/setup/status")
                .header("x-zxs-bootstrap", &key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status: Value = json_codec::from_slice(
        &http_routing::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(status["state"], "empty");
    for _ in 0..100 {
        if !backend.management_status()["recovering"].as_bool().unwrap() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let response=app.clone().oneshot(Request::builder().method("POST").uri("/api/setup/initialize")
        .header("host","127.0.0.1:7400").header("origin","http://127.0.0.1:7400").header("x-zxs-bootstrap",&key).header("x-csrf-token",status["csrf_token"].as_str().unwrap()).header("content-type","application/json")
        .body(Body::from(json_codec::to_vec(&json!({"mode":"development","disk_ids":ids,"username":"admin","password":"a-long-test-password"})).unwrap())).unwrap()).await.unwrap();
    let code = response.status();
    let value: Value = json_codec::from_slice(
        &http_routing::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(code, StatusCode::OK, "{value:?}");
    assert_eq!(value["restart_required"], false);
    assert!(data.join("management-authority.json").exists());
    assert!(management.join("management-auth.json").exists());
    assert_eq!(
        request(
            &app,
            "/api/auth/login",
            None,
            Some(json!({"username":"admin","password":"a-long-test-password"}))
        )
        .await
        .0,
        StatusCode::OK
    );

    for _ in 0..200 {
        if !backend.management_status()["recovering"].as_bool().unwrap() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(backend.management_status()["state"], "ready");
    let config: storage_contract::Config =
        json_codec::from_slice(&std::fs::read(data.join("config.json")).unwrap()).unwrap();
    assert_eq!(config.disks.len(), ids.len());
    assert_eq!(
        config.disks.iter().filter(|d| d.metadata_member).count(),
        ids.len()
    );
    let (_, login) = request(
        &app,
        "/api/auth/login",
        None,
        Some(json!({"username":"admin","password":"a-long-test-password"})),
    )
    .await;
    let token = login["token"].as_str().unwrap();
    let (code, repository) = request(&app, "/api/repositories", Some(token), Some(json!({"name":"single-copy","primary_slot_id":"P01","target_copies":1,"operation_id":storage_contract::new_id("op")}))).await;
    assert_eq!(code, StatusCode::OK, "{repository:?}");
    if remove_data_disk {
        drop(app);
        drop(backend);
        std::fs::remove_dir_all(root.path().join(ids[0])).unwrap();
        let (backend, auth) = open_service(&data, &management).unwrap();
        let app = router(backend.clone(), auth, ServerConfig::loopback(&management)).unwrap();
        let (code, login) = request(
            &app,
            "/api/auth/login",
            None,
            Some(json!({"username":"admin","password":"a-long-test-password"})),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        let token = login["token"].as_str().unwrap();
        assert_eq!(
            request(&app, "/healthz", None, None).await.0,
            StatusCode::OK
        );
        assert_eq!(
            request(&app, "/api/admin/accounts", Some(token), None)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            request(&app, "/api/system/storage", Some(token), None)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            request(&app, "/api/repositories", Some(token), None)
                .await
                .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(!root.path().join(ids[0]).exists());
    }
}

#[tokio::test]
async fn account_http_lifecycle_remains_authoritative_without_storage() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = root.path().join("management");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(data.join("config.json"), "broken").unwrap();
    drop(AuthStore::initialize_management(&management, "admin", "a-long-test-password").unwrap());
    let (backend, auth) = open_service(&data, &management).unwrap();
    let mut config = ServerConfig::loopback(&management);
    let web = root.path().join("web");
    std::fs::create_dir(&web).unwrap();
    std::fs::write(web.join("index.html"), "management shell").unwrap();
    config.web_dir = Some(web);
    let app = router(backend, auth, config).unwrap();
    let asset = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(asset.status(), StatusCode::OK);
    let (_, login) = request(
        &app,
        "/api/auth/login",
        None,
        Some(json!({"username":"admin","password":"a-long-test-password"})),
    )
    .await;
    let token = login["token"].as_str().unwrap();
    assert_eq!(request(&app,"/api/admin/accounts",Some(token),Some(json!({"username":"reader","password":"a-reader-test-password","repositories":["repo-one"]}))).await.0,StatusCode::OK);
    let (_, reader) = request(
        &app,
        "/api/auth/login",
        None,
        Some(json!({"username":"reader","password":"a-reader-test-password"})),
    )
    .await;
    let old = reader["token"].as_str().unwrap();
    let (status, list) = request(&app, "/api/admin/accounts", Some(token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["accounts"].as_array().unwrap().len(), 2);
    assert_eq!(
        request(
            &app,
            "/api/admin/accounts",
            Some(token),
            Some(json!({"username":"reader","repositories":["repo-two"]}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/auth/me", Some(old), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (_, reader) = request(
        &app,
        "/api/auth/login",
        None,
        Some(json!({"username":"reader","password":"a-reader-test-password"})),
    )
    .await;
    let renewed = reader["token"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            "/api/admin/accounts",
            Some(token),
            Some(json!({"username":"reader","disabled":true}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/auth/me", Some(renewed), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, error) = request(
        &app,
        "/api/admin/accounts",
        Some(token),
        Some(json!({"username":"admin","disabled":true})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "LAST_ADMIN_REQUIRED");
}

#[test]
fn managed_admission_never_falls_back_to_stale_disaster_recovery_grants() {
    use std::sync::{Arc, Mutex};
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = root.path().join("management");
    let engine = storage_engine::Engine::initialize(
        &data,
        storage_contract::Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]),
    )
    .unwrap();
    let backend = Arc::new(crate::engine_backend::EngineBackend::new(engine));
    let mut legacy = AuthStore::initialize(&data, "admin", "a-long-test-password").unwrap();
    legacy
        .upsert_account(
            "reader",
            Some("a-reader-test-password"),
            true,
            Default::default(),
            false,
        )
        .unwrap();
    drop(backend.protect_auth(legacy).unwrap());
    let mut primary =
        AuthStore::initialize_management(&management, "admin", "a-long-test-password").unwrap();
    primary
        .upsert_account(
            "reader",
            Some("a-reader-test-password"),
            false,
            Default::default(),
            true,
        )
        .unwrap();
    let primary = Arc::new(Mutex::new(primary));
    backend
        .set_management_auth(Arc::downgrade(&primary))
        .unwrap();
    let caller = crate::auth::Identity {
        admission_session: None,
        smb_binding: None,
        principal: "reader".into(),
        admin: true,
        repositories: Default::default(),
    };
    assert_eq!(
        backend
            .call(crate::BackendAction::Repositories, &caller, json!({}))
            .unwrap_err()
            .code,
        "UNAUTHORIZED"
    );
    let admin = primary
        .lock()
        .unwrap()
        .identity_for_principal("admin")
        .unwrap();
    drop(primary);
    assert_eq!(
        backend
            .call(crate::BackendAction::Repositories, &admin, json!({}))
            .unwrap_err()
            .code,
        "UNAUTHORIZED"
    );
}

#[test]
fn stale_readiness_retry_waits_for_existing_engine_holders() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = root.path().join("management");
    let engine = storage_engine::Engine::initialize(
        &data,
        storage_contract::Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]),
    )
    .unwrap();
    let held = std::sync::Arc::new(crate::engine_backend::EngineBackend::new(engine));
    let (backend, _auth) = open_service(&data, &management).unwrap();
    {
        let mut state = backend.inner.state.lock().unwrap();
        state.backend = Some(held.clone());
        state.status = super::StorageState::Ready;
        state.admitted = true;
        state.observed_at = std::time::Instant::now() - std::time::Duration::from_secs(16);
    }
    assert_eq!(backend.management_status()["state"], "unavailable");
    let error = backend.inner.start_recovery(true).unwrap_err();
    assert_eq!(error.code, "STORAGE_DRAINING");
    assert!(error.retryable);
    assert!(
        !backend
            .inner
            .recovering
            .load(std::sync::atomic::Ordering::Acquire)
    );
    assert!(!backend.inner.state.lock().unwrap().admitted);
}

#[tokio::test]
async fn web_completion_requires_bound_pristine_pre_authority_setup_proof() {
    use std::os::unix::fs::PermissionsExt;
    for (interrupted_owned_setup, authority_already_claimed) in
        [(true, false), (true, true), (false, false)]
    {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("pool");
        let management = data.join("management");
        let mut disks = std::collections::BTreeMap::new();
        for id in ["one", "two", "three"] {
            let path = root.path().join(id);
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            disks.insert(id.to_owned(), path);
        }
        let (backend, auth) =
            open_service_with_disks(&data, &management, disks.clone(), true).unwrap();
        let key = "a".repeat(64);
        std::fs::write(management.join("bootstrap.key"), &key).unwrap();
        std::fs::set_permissions(
            management.join("bootstrap.key"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        if interrupted_owned_setup {
            let service = crate::setup_recovery::SetupRecovery::new(
                data.clone(),
                management.clone(),
                disks.clone(),
            )
            .unwrap()
            .allow_development_mode();
            let permit = service.authorize_bootstrap(&key).unwrap();
            let request=json_codec::from_value(json!({"mode":"development","disk_ids":["one","two","three"],"username":"admin","password":"a-long-test-password"})).unwrap();
            let error = service
                .initialize(&permit, request, |_engine, _, _| {
                    Err(crate::ApiError::new(
                        500,
                        "TEST_INSTALL_FAILURE",
                        "injected failure before authority publication",
                    ))
                })
                .unwrap_err();
            assert_eq!(error.code, "TEST_INSTALL_FAILURE");
        } else {
            drop(
                storage_engine::Engine::initialize(
                    &data,
                    storage_contract::Config::development([
                        disks["one"].clone(),
                        disks["two"].clone(),
                        disks["three"].clone(),
                    ]),
                )
                .unwrap(),
            );
        }
        if authority_already_claimed {
            claim_authority(&data).unwrap();
        }
        let completion_expected = interrupted_owned_setup && !authority_already_claimed;
        assert!(!management.join("management-auth.json").exists());
        assert_eq!(
            data.join("management-authority.json").exists(),
            authority_already_claimed
        );
        let app = router(backend.clone(), auth, ServerConfig::loopback(&management)).unwrap();
        for _ in 0..100 {
            if !backend.management_status()["recovering"].as_bool().unwrap() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/setup/status")
                    .header("x-zxs-bootstrap", &key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let status: Value = json_codec::from_slice(
            &http_routing::body::to_bytes(response.into_body(), 65536)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            status["completion_available"], completion_expected,
            "{status:?}"
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/setup/complete-initialization")
                    .header("host", "127.0.0.1:7400")
                    .header("origin", "http://127.0.0.1:7400")
                    .header("x-zxs-bootstrap", &key)
                    .header("x-csrf-token", status["csrf_token"].as_str().unwrap())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json_codec::to_vec(
                            &json!({"username":"admin","password":"a-long-test-password"}),
                        )
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let code = response.status();
        let value: Value = json_codec::from_slice(
            &http_routing::body::to_bytes(response.into_body(), 65536)
                .await
                .unwrap(),
        )
        .unwrap();
        if completion_expected {
            assert_eq!(code, StatusCode::OK, "{value:?}");
            assert_eq!(value["completed"], true);
            wait_for_ready(&backend).await;
            assert!(data.join("management-authority.json").exists());
            assert!(management.join("management-auth.json").exists());
            assert_eq!(
                request(
                    &app,
                    "/api/auth/login",
                    None,
                    Some(json!({"username":"admin","password":"a-long-test-password"}))
                )
                .await
                .0,
                StatusCode::OK
            );
        } else {
            assert!(!code.is_success(), "{value:?}");
            assert!(!management.join("management-auth.json").exists());
            assert_eq!(
                data.join("management-authority.json").exists(),
                authority_already_claimed
            );
        }
    }
}

async fn bootstrap_json(
    app: &Router,
    key: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/setup/status")
                .header("x-zxs-bootstrap", key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status.status(), StatusCode::OK);
    let status: Value = json_codec::from_slice(
        &http_routing::body::to_bytes(status.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    let Some(body) = body else {
        return (StatusCode::OK, status);
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("host", "127.0.0.1:7400")
                .header("origin", "http://127.0.0.1:7400")
                .header("x-zxs-bootstrap", key)
                .header("x-csrf-token", status["csrf_token"].as_str().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(json_codec::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let code = response.status();
    let value = json_codec::from_slice(
        &http_routing::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    (code, value)
}

#[tokio::test]
async fn preset_web_setup_is_lazy_digest_bound_and_preserves_custom_configuration() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("pool");
    let management = data.join("management");
    let mut disks = std::collections::BTreeMap::new();
    for id in ["one", "two", "three", "four"] {
        let path = root.path().join(id);
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        disks.insert(id.to_owned(), path);
    }
    let mut config = storage_contract::Config::development([
        disks["one"].clone(),
        disks["two"].clone(),
        disks["three"].clone(),
    ]);
    let mut extra = config.disks[2].clone();
    extra.slot_id = "D01".into();
    extra.level = 4;
    extra.upstream_slot_id = Some("C01".into());
    extra.disk_id = storage_contract::new_id("disk");
    extra.root = disks["four"].clone();
    extra.metadata_member = false;
    config.disks.push(extra);
    config.max_workspaces = 7;
    config.checksum_chunk_bytes = 1_048_576;
    let preset = root.path().join("custom-config.json");
    std::fs::write(&preset, json_codec::to_vec(&config).unwrap()).unwrap();
    std::fs::set_permissions(&preset, std::fs::Permissions::from_mode(0o600)).unwrap();
    let (backend, auth) = open_service_with_options(
        &data,
        &management,
        ManagementOptions {
            setup_disks: disks,
            setup_configs: std::collections::BTreeMap::from([
                ("custom".into(), preset.clone()),
                (
                    "offline".into(),
                    root.path().join("unavailable-preset.json"),
                ),
            ]),
            allow_development_setup: true,
        },
    )
    .unwrap();
    let key = "b".repeat(64);
    std::fs::write(management.join("bootstrap.key"), &key).unwrap();
    std::fs::set_permissions(
        management.join("bootstrap.key"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let app = router(backend.clone(), auth, ServerConfig::loopback(&management)).unwrap();
    assert_eq!(
        request(&app, "/healthz", None, None).await.0,
        StatusCode::OK
    );
    for _ in 0..100 {
        if !backend.management_status()["recovering"].as_bool().unwrap() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let (_, status) = bootstrap_json(&app, &key, "/api/setup/status", None).await;
    assert_eq!(status["config_presets"], json!(["custom", "offline"]));
    let (code, preview) = bootstrap_json(
        &app,
        &key,
        "/api/setup/preset-preview",
        Some(json!({"preset_id":"custom"})),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{preview:?}");
    assert!(!preview.to_string().contains(root.path().to_str().unwrap()));
    assert_eq!(
        preview["configuration"]["disks"].as_array().unwrap().len(),
        4
    );
    config.max_workspaces = 8;
    std::fs::write(&preset, json_codec::to_vec(&config).unwrap()).unwrap();
    let (code,_)=bootstrap_json(&app,&key,"/api/setup/initialize-preset",Some(json!({"preset_id":"custom","config_digest":preview["config_digest"],"username":"admin","password":"a-long-test-password"}))).await;
    assert!(!code.is_success());
    assert!(!data.join("config.json").exists());
    assert!(!management.join("management-auth.json").exists());
    let (code, preview) = bootstrap_json(
        &app,
        &key,
        "/api/setup/preset-preview",
        Some(json!({"preset_id":"custom"})),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let (code,value)=bootstrap_json(&app,&key,"/api/setup/initialize-preset",Some(json!({"preset_id":"custom","config_digest":preview["config_digest"],"username":"admin","password":"a-long-test-password"}))).await;
    assert_eq!(code, StatusCode::OK, "{value:?}");
    wait_for_ready(&backend).await;
    let persisted: Value =
        json_codec::from_slice(&std::fs::read(data.join("config.json")).unwrap()).unwrap();
    assert_eq!(persisted, json_codec::to_value(config).unwrap());
    assert_eq!(
        request(
            &app,
            "/api/auth/login",
            None,
            Some(json!({"username":"admin","password":"a-long-test-password"}))
        )
        .await
        .0,
        StatusCode::OK
    );
}

async fn wait_for_ready(backend: &ManagementBackend) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if backend.management_status()["state"] == "ready" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("successful installation must reopen storage without a restart");
}
