use super::*;
use crate::{ApiError, Backend, BackendAction, Content, auth::Identity};
use http_routing::{Router, routing::get};
use json_codec::{Value, json};
use smb_protocol::protocol::{
    FrameStream, PROTOCOL, Request, Response, WRITE_RESERVATION_CAPABILITY,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::Path,
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Semaphore,
};

struct BackendProbe;
impl Backend for BackendProbe {
    fn call(&self, action: BackendAction, _: &Identity, _: Value) -> Result<Value, ApiError> {
        match action {
            BackendAction::Repository => Ok(json!({"main_branch_id":"test-branch"})),
            BackendAction::Capabilities => Ok(json!({"max_upload_bytes":"4096"})),
            _ => Err(ApiError::unsupported()),
        }
    }
    fn upload(&self, _: &Identity, _: &str, _: &str, _: &str, _: &Path) -> Result<Value, ApiError> {
        Err(ApiError::unsupported())
    }
    fn content(&self, _: &Identity, _: &str) -> Result<Content, ApiError> {
        Err(ApiError::unsupported())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_closes_idle_smb_and_http_listener_while_http_request_is_stalled() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.path().join("smb.sock");
    let bridge_socket = socket.clone();
    let identities = BTreeMap::from([(
        "alice".to_owned(),
        Identity {
            admission_session: None,
            principal: "alice".into(),
            admin: false,
            repositories: BTreeSet::from(["test-repository".into()]),
        },
    )]);
    let (stop, shutdown) = oneshot::channel();
    let stalled = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let app = Router::new().route(
        "/stall",
        get({
            let stalled = stalled.clone();
            let release = release.clone();
            move || {
                let stalled = stalled.clone();
                let release = release.clone();
                async move {
                    stalled.add_permits(1);
                    release.acquire().await.unwrap().forget();
                    "finished"
                }
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(run(
        |stopped| async move {
            http_routing::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = stopped.await;
                })
                .await
        },
        async move {
            crate::smb_rpc::serve(
                &bridge_socket,
                smb_protocol::protocol::effective_uid(),
                identities,
                Arc::new(BackendProbe),
                crate::SpoolLimits::default(),
                4096,
            )
            .await
            .unwrap();
        },
        async move {
            let _ = shutdown.await;
        },
    ));
    let mut http = tokio::net::TcpStream::connect(address).await.unwrap();
    http.write_all(b"GET /stall HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    tokio::time::timeout(DEADLINE, stalled.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    tokio::time::timeout(DEADLINE, async {
        while !socket.exists() {
            assert!(!server.is_finished(), "bridge failed before binding");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let mut idle = tokio::task::spawn_blocking({
        let socket = socket.clone();
        move || {
            let mut wire =
                FrameStream::with_timeout(UnixStream::connect(socket).unwrap(), DEADLINE).unwrap();
            wire.send(
                &Request::Hello {
                    protocol: PROTOCOL.into(),
                    capabilities: vec![WRITE_RESERVATION_CAPABILITY.into()],
                    username: "alice".into(),
                    repository_id: "test-repository".into(),
                },
                None,
            )
            .unwrap();
            let (reply, _) = wire.receive::<Response>().unwrap();
            assert!(matches!(reply, Response::Ok { .. }));
            wire.0
        }
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    tokio::task::spawn_blocking(move || {
        let mut byte = [0];
        assert_eq!(
            idle.read(&mut byte).unwrap(),
            0,
            "idle SMB session must close"
        );
    })
    .await
    .unwrap();
    tokio::time::timeout(DEADLINE, async {
        while socket.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("SMB socket must be removed before HTTP drain");
    assert!(UnixStream::connect(&socket).is_err());
    tokio::time::timeout(DEADLINE, async {
        while tokio::net::TcpStream::connect(address).await.is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        !server.is_finished(),
        "stalled admitted HTTP request must still drain"
    );
    release.add_permits(1);
    let mut response = String::new();
    tokio::time::timeout(DEADLINE, http.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.ends_with("finished"), "{response}");
    tokio::time::timeout(DEADLINE, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
