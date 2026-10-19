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
