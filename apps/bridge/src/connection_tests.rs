use super::*;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

fn client(host: &str) -> Client {
    Client::connect(&format!("https://{host}")).unwrap()
}

#[test]
fn failed_or_superseded_logins_preserve_the_active_connection() {
    let mut state = connection::Connection::default();
    let first = state.begin();
    assert!(state.install(first, client("old.example")));
    let (_, revision) = state.snapshot().unwrap();
    let older_attempt = state.begin();
    state.begin(); // A newer login failed; neither pending attempt replaces the old session.
    assert!(!state.install(older_attempt, client("stale.example")));
    assert!(state.current(revision));
    assert_eq!(state.snapshot().unwrap().0.origin(), "https://old.example/");
    let pending = state.begin();
    state.clear();
    assert!(!state.install(pending, client("stale.example")));
    assert!(state.snapshot().is_none());
}

#[tokio::test]
async fn an_old_logout_response_cannot_disconnect_a_new_session() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, received) = oneshot::channel();
    let (release, wait) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
            assert!(headers.len() < 8192);
        }
        sent.send(()).unwrap();
        wait.await.unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
            .await
            .unwrap();
    });
    let bridge = Arc::new(Bridge::default());
    {
        let mut state = bridge.connection.write().await;
        let attempt = state.begin();
        assert!(state.install(attempt, Client::connect(&endpoint).unwrap()));
    }
    let caller = bridge.clone();
    let request =
        tokio::spawn(async move { caller.request("POST", "/v1/auth/logout", None).await });
    received.await.unwrap();
    {
        let mut state = bridge.connection.write().await;
        let attempt = state.begin();
        assert!(state.install(attempt, client("new.example")));
    }
    release.send(()).unwrap();
    assert_eq!(request.await.unwrap().unwrap_err().code, "SESSION_CHANGED");
    assert_eq!(
        bridge.snapshot().await.unwrap().0.origin(),
        "https://new.example/"
    );
    server.await.unwrap();
}

#[test]
fn only_a_destination_collision_is_a_cache_reuse_candidate() {
    let exists: BridgeError = Error::Io(std::io::ErrorKind::AlreadyExists.into()).into();
    let denied: BridgeError = Error::Io(std::io::ErrorKind::PermissionDenied.into()).into();
    assert_eq!(exists.code, "DESTINATION_EXISTS");
    assert_eq!(denied.code, "LOCAL_IO_ERROR");
}

#[tokio::test]
async fn an_old_cache_verification_error_is_isolated_from_the_new_session() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, received) = oneshot::channel();
    let (release, wait) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
            assert!(headers.len() < 8192);
        }
        sent.send(()).unwrap();
        wait.await.unwrap();
        stream
            .write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
            )
            .await
            .unwrap();
    });
    let bridge = Arc::new(Bridge::default());
    {
        let mut state = bridge.connection.write().await;
        let attempt = state.begin();
        assert!(state.install(attempt, Client::connect(&endpoint).unwrap()));
    }
    let caller = bridge.clone();
    let request = tokio::spawn(async move {
        caller
            .verify_cached("ver_test", Path::new("unused-cache-entry"))
            .await
    });
    received.await.unwrap();
    {
        let mut state = bridge.connection.write().await;
        let attempt = state.begin();
        assert!(state.install(attempt, client("new.example")));
    }
    release.send(()).unwrap();
    assert_eq!(request.await.unwrap().unwrap_err().code, "SESSION_CHANGED");
    assert_eq!(
        bridge.snapshot().await.unwrap().0.origin(),
        "https://new.example/"
    );
    server.await.unwrap();
}

async fn logout_server(
    statuses: Vec<&'static str>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for status in statuses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                headers.push(stream.read_u8().await.unwrap());
                assert!(headers.len() < 8192);
            }
            requests.push(String::from_utf8(headers).unwrap());
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
        requests
    });
    (endpoint, server)
}

async fn connected_bridge(endpoint: &str) -> Bridge {
    let bridge = Bridge::default();
    let mut client = Client::connect(endpoint).unwrap();
    client.set_token(Some("test-session".into()));
    {
        let mut state = bridge.connection.write().await;
        let attempt = state.begin();
        assert!(state.install(attempt, client));
    }
    bridge
}

#[tokio::test]
async fn failed_logout_can_retry_with_the_same_credential_then_revokes_pending_login() {
    let (endpoint, server) = logout_server(vec!["503 Service Unavailable", "200 OK"]).await;
    let bridge = connected_bridge(&endpoint).await;
    let (_, revision) = bridge.snapshot().await.unwrap();
    let pending_login = bridge.connection.write().await.begin();
    assert!(
        bridge
            .request("POST", "/v1/auth/logout", None)
            .await
            .is_err()
    );
    assert!(bridge.connection.read().await.current(revision));
    bridge
        .request("POST", "/v1/auth/logout?source=app", None)
        .await
        .unwrap();
    assert_eq!(bridge.snapshot().await.err().unwrap().code, "NOT_CONNECTED");
    assert!(
        !bridge
            .connection
            .write()
            .await
            .install(pending_login, client("stale.example"))
    );
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer test-session\r\n")
        );
    }
}

#[tokio::test]
async fn rejected_logout_method_does_not_drop_session() {
    let (endpoint, server) = logout_server(vec!["405 Method Not Allowed"]).await;
    let bridge = connected_bridge(&endpoint).await;
    assert!(
        bridge
            .request("GET", "/v1/auth/logout", None)
            .await
            .is_err()
    );
    assert!(bridge.snapshot().await.is_ok());
    server.await.unwrap();
}

#[tokio::test]
async fn logout_unauthorized_confirms_session_is_already_invalid() {
    let (endpoint, server) = logout_server(vec!["401 Unauthorized"]).await;
    let bridge = connected_bridge(&endpoint).await;
    assert_eq!(
        bridge
            .request("POST", "/v1/auth/logout", None)
            .await
            .unwrap_err()
            .status,
        401
    );
    assert!(bridge.snapshot().await.is_err());
    server.await.unwrap();
}

#[tokio::test]
async fn generic_login_cannot_return_credentials_or_create_an_untracked_session() {
    let bridge = Bridge::default();
    for route in ["/v1/auth/login", "/v1/auth/login?source=app"] {
        assert_eq!(
            bridge.request("POST", route, None).await.unwrap_err().code,
            "NATIVE_LOGIN_REQUIRED"
        );
    }
}

#[tokio::test]
async fn restored_native_session_does_not_expose_credentials_to_webview() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = connected_bridge(&format!("http://{}", listener.local_addr().unwrap())).await;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
        }
        let body = json!({"principal":"alice","admin":false,"repositories":["repo"],"csrf_token":"secret-csrf","token":"secret-bearer"}).to_string();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let result = bridge
        .request("GET", "/v1/auth/me?source=app", None)
        .await
        .unwrap();
    assert_eq!(
        result,
        json!({"principal":"alice","admin":false,"repositories":["repo"]})
    );
    assert!(bridge.snapshot().await.is_ok());
    server.await.unwrap();
}

async fn envelope_server(
    responses: Vec<(u16, Value)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                headers.push(stream.read_u8().await.unwrap());
                assert!(headers.len() < 8192);
            }
            let headers = String::from_utf8(headers).unwrap();
            let length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            assert!(length < 8192);
            let mut request_body = vec![0; length];
            stream.read_exact(&mut request_body).await.unwrap();
            requests.push(format!(
                "{headers}{}",
                String::from_utf8(request_body).unwrap()
            ));
            let body = body.to_string();
            stream.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
        requests
    });
    (endpoint, server)
}

#[tokio::test]
async fn real_login_keeps_secrets_native_and_failed_reauthentication_preserves_session() {
    let (endpoint, server) = envelope_server(vec![
        (200, json!({"token":"native-bearer", "csrf_token":"native-csrf", "principal":"alice", "admin":false, "expires_at":"9007199254740993"})),
        (401, json!({"error":{"code":"INVALID_CREDENTIALS","message":"invalid login","retryable":false}})),
        (200, json!({"ok":true})),
    ]).await;
    let bridge = Bridge::default();
    assert_eq!(
        bridge
            .connect(&endpoint, "alice", "synthetic-password")
            .await
            .unwrap(),
        json!({"principal":"alice","admin":false,"expires_at":"9007199254740993"})
    );
    let (_, revision) = bridge.snapshot().await.unwrap();
    let error = bridge
        .connect(&endpoint, "alice", "wrong-password")
        .await
        .unwrap_err();
    assert_eq!(error.code, "INVALID_CREDENTIALS");
    assert_eq!(error.status, 401);
    assert!(bridge.connection.read().await.current(revision));
    assert_eq!(
        bridge
            .request("GET", "/v1/system/status", None)
            .await
            .unwrap(),
        json!({"ok":true})
    );
    let requests = server.await.unwrap();
    for request in &requests[..2] {
        assert!(request.starts_with("POST /v1/auth/login HTTP/1.1\r\n"));
        assert!(!request.to_lowercase().contains("authorization:"));
    }
    assert!(requests[0].contains("synthetic-password"));
    assert!(
        requests[2]
            .to_lowercase()
            .contains("authorization: bearer native-bearer\r\n")
    );
    assert!(!requests[2].contains("native-csrf"));
}

#[tokio::test]
async fn expired_session_errors_survive_request_upload_and_download_boundaries() {
    let envelope = json!({"error":{"code":"SESSION_EXPIRED","message":"sign in again","retryable":false,"request_id":"req_expired"}});
    let (endpoint, server) = envelope_server(vec![
        (401, envelope.clone()),
        (401, envelope.clone()),
        (401, envelope),
    ])
    .await;
    let bridge = connected_bridge(&endpoint).await;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    std::fs::write(&source, b"upload source").unwrap();
    let errors = [
        bridge
            .request("GET", "/v1/system/status", None)
            .await
            .unwrap_err(),
        bridge
            .upload("workspace_test", "item_test", "file", &source)
            .await
            .unwrap_err(),
        bridge
            .download("version_test", &directory.path().join("download"))
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.status, 401);
        assert_eq!(error.code, "SESSION_EXPIRED");
        assert_eq!(error.message, "sign in again (request req_expired)");
        assert!(!error.retryable);
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read(source).unwrap(), b"upload source");
    for request in server.await.unwrap() {
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer test-session\r\n")
        );
    }
}

#[tokio::test]
async fn forbidden_resource_keeps_the_session_and_preserves_error_envelope() {
    let (endpoint, server) = envelope_server(vec![
        (403, json!({"error":{"code":"FORBIDDEN","message":"repository denied","retryable":false,"request_id":"req_denied"}})),
        (200, json!({"ok":true})),
    ]).await;
    let bridge = connected_bridge(&endpoint).await;
    let (_, revision) = bridge.snapshot().await.unwrap();
    let error = bridge
        .request("GET", "/v1/repositories/private", None)
        .await
        .unwrap_err();
    assert_eq!(error.status, 403);
    assert_eq!(error.code, "FORBIDDEN");
    assert_eq!(error.message, "repository denied (request req_denied)");
    assert!(!error.retryable);
    assert!(bridge.connection.read().await.current(revision));
    bridge
        .request("GET", "/v1/system/status", None)
        .await
        .unwrap();
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn password_change_revokes_connection_only_after_confirmed_reauthentication_requirement() {
    let (endpoint, server) = envelope_server(vec![
        (
            200,
            json!({"changed":false,"reauthentication_required":true}),
        ),
        (
            200,
            json!({"changed":true,"reauthentication_required":true}),
        ),
    ])
    .await;
    let bridge = connected_bridge(&endpoint).await;
    bridge
        .request("POST", "/v1/auth/password", Some(&json!({})))
        .await
        .unwrap();
    assert!(bridge.snapshot().await.is_ok());
    let pending = bridge.connection.write().await.begin();
    bridge
        .request("POST", "/v1/auth/password?source=app", Some(&json!({})))
        .await
        .unwrap();
    assert_eq!(bridge.snapshot().await.err().unwrap().code, "NOT_CONNECTED");
    assert!(
        !bridge
            .connection
            .write()
            .await
            .install(pending, client("stale.example"))
    );
    assert_eq!(server.await.unwrap().len(), 2);
}
