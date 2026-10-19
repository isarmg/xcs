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
