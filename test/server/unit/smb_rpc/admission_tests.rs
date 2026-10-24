//! Per-principal fairness while authenticated connections remain idle.
use super::*;
use std::{net::Shutdown, os::unix::net::UnixStream, thread::JoinHandle, time::Duration};

struct Connection {
    wire: FrameStream,
    cancel: UnixStream,
    worker: JoinHandle<io::Result<()>>,
}

fn connect(shared: Arc<Shared>, username: &str, repository: &str) -> (Connection, Response) {
    let (client, server) = UnixStream::pair().unwrap();
    let cancel = server.try_clone().unwrap();
    let worker =
        std::thread::spawn(move || run_connection(FrameStream::new(server).unwrap(), shared));
    let mut wire = FrameStream::with_timeout(client, Duration::from_secs(2)).unwrap();
    wire.send(
        &Request::Hello {
            protocol: PROTOCOL.into(),
            capabilities: REQUIRED_CAPABILITIES
                .iter()
                .map(|value| (*value).into())
                .collect(),
            username: username.into(),
            repository_id: repository.into(),
        },
        None,
    )
    .unwrap();
    let (response, descriptor) = wire.receive::<Response>().unwrap();
    assert!(descriptor.is_none());
    (
        Connection {
            wire,
            cancel,
            worker,
        },
        response,
    )
}

fn disconnect(connection: Connection, cancel: bool) {
    if cancel {
        // Same cancellation mechanism as the listening service's SocketGuard.
        connection.cancel.shutdown(Shutdown::Both).unwrap();
    }
    drop(connection.wire);
    connection.worker.join().unwrap().unwrap();
}

#[test]
fn sole_account_session_cap_cannot_be_bypassed_by_another_samba_name() {
    let (_temp, shared, _backend, _branch) = fixture();
    let repository = repository(&shared);
    let mut idle = Vec::new();
    for _ in 0..MAX_SESSIONS_PER_ACCOUNT {
        let (connection, response) = connect(shared.clone(), "alice", &repository);
        assert!(matches!(response, Response::Ok { .. }));
        idle.push(connection);
    }
    let (rejected, response) = connect(shared.clone(), "alice", &repository);
    assert!(matches!(response, Response::Error { code, .. } if code == "RESOURCE_LIMIT_EXCEEDED"));
    disconnect(rejected, false);
    for username in ["alias-alice", "bob", "", "anonymous", "guest"] {
        let (rejected, response) = connect(shared.clone(), username, &repository);
        assert!(matches!(response, Response::Error { code, .. } if code == "FORBIDDEN"));
        disconnect(rejected, false);
    }
    assert_eq!(
        shared.sessions_per_account.lock().unwrap()["alice"],
        MAX_SESSIONS_PER_ACCOUNT
    );
    disconnect(idle.pop().unwrap(), false);
    let (replacement, response) = connect(shared.clone(), "alice", &repository);
    assert!(matches!(response, Response::Ok { .. }));
    idle.push(replacement);
    for connection in idle {
        disconnect(connection, true);
    }
    assert!(shared.sessions_per_account.lock().unwrap().is_empty());
}

#[test]
fn failed_repository_admission_does_not_leak_account_session_slots() {
    let (_temp, shared, _backend, _branch) = fixture();
    for _ in 0..=MAX_SESSIONS_PER_ACCOUNT {
        let (rejected, response) = connect(shared.clone(), "alice", &new_id("repo"));
        assert!(
            matches!(response, Response::Error { code, .. } if code != "RESOURCE_LIMIT_EXCEEDED")
        );
        disconnect(rejected, false);
        assert!(shared.sessions_per_account.lock().unwrap().is_empty());
    }
}

#[test]
fn configured_samba_account_binds_the_current_single_authority_and_fails_closed_during_recovery() {
    let (temp, mut shared, _backend, _branch) = fixture();
    let repository = repository(&shared);
    Arc::get_mut(&mut shared).unwrap().username = "samba-owner".into();
    let (wrong, response) = connect(shared.clone(), "alice", &repository);
    assert!(matches!(response, Response::Error { code, .. } if code == "FORBIDDEN"));
    disconnect(wrong, false);
    let (mut live, response) = connect(shared.clone(), "samba-owner", &repository);
    assert!(matches!(response, Response::Ok { result } if result["principal"] == "alice"));

    *shared.auth.lock().unwrap() = AuthStore::unavailable(&temp.path().join("recovering"));
    live.wire
        .send(
            &Request::Stat {
                path: "report.txt".into(),
            },
            None,
        )
        .unwrap();
    assert!(
        matches!(live.wire.receive::<Response>().unwrap().0, Response::Error { code, .. } if code == "RECOVERY_REQUIRED")
    );
    let (rejected, response) = connect(shared.clone(), "samba-owner", &repository);
    assert!(matches!(response, Response::Error { code, .. } if code == "RECOVERY_REQUIRED"));
    disconnect(rejected, false);
    disconnect(live, false);
    assert!(shared.sessions_per_account.lock().unwrap().is_empty());

    *shared.auth.lock().unwrap() = AuthStore::initialize(
        &temp.path().join("restored-auth"),
        "restored-owner",
        "long-test-password",
    )
    .unwrap();
    let (restored, response) = connect(shared.clone(), "samba-owner", &repository);
    assert!(matches!(response, Response::Ok { result } if result["principal"] == "restored-owner"));
    disconnect(restored, false);
}
