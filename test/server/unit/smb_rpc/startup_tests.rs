use super::*;

#[tokio::test(flavor = "current_thread")]
async fn startup_io_does_not_pin_timers_or_health_on_a_single_worker() {
    use service_stack::ServiceExt;
    use std::time::Duration;
    let started = Arc::new(tokio::sync::Semaphore::new(0));
    let (release, wait) = std::sync::mpsc::channel();
    let work = tokio::spawn(startup_io({
        let started = started.clone();
        move || {
            started.add_permits(1);
            wait.recv_timeout(Duration::from_secs(3))
                .map_err(io::Error::other)?;
            Ok(())
        }
    }));
    started.acquire().await.unwrap().forget();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!work.is_finished());
    let health = http_routing::Router::new()
        .route("/healthz", http_routing::routing::get(|| async { "alive" }))
        .oneshot(
            http_routing::http::Request::builder()
                .uri("/healthz")
                .body(http_routing::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), http_routing::http::StatusCode::OK);
    release.send(()).unwrap();
    work.await.unwrap().unwrap();
}

#[tokio::test]
async fn startup_rejects_nonprivate_and_aliased_directories_before_binding() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("public");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
    let socket = parent.join("rpc.sock");
    let error = startup_io(move || prepare_socket(&socket, crate::SpoolLimits::default()))
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(!parent.join("smb-spool").exists());
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&parent, &alias).unwrap();
    let socket = alias.join("rpc.sock");
    let error = startup_io(move || prepare_socket(&socket, crate::SpoolLimits::default()))
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

// Real AF_UNIX binding/registration is exercised by full-feature CI. Local
// sandboxes denying Unix sockets must not bypass that restriction to run this.
#[cfg(feature = "core")]
#[tokio::test]
async fn bind_and_register_prepared_socket_preserves_permissions_and_identity_cleanup() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.path().join("rpc.sock");
    let path = socket.clone();
    let prepared = startup_io(move || prepare_socket(&path, crate::SpoolLimits::default()))
        .await
        .unwrap();
    assert_eq!(fs::symlink_metadata(&socket).unwrap().mode() & 0o777, 0o600);
    let listener = tokio::net::UnixListener::from_std(prepared.listener).unwrap();
    let client = tokio::net::UnixStream::connect(&socket).await.unwrap();
    let (server, _) = listener.accept().await.unwrap();
    drop((client, server, listener));
    drop(prepared.guard);
    assert!(!socket.exists());
}
