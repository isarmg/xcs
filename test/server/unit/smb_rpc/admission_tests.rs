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
            capabilities: vec![WRITE_RESERVATION_CAPABILITY.into()],
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
fn idle_session_cap_is_shared_by_aliases_but_not_other_accounts() {
    let (_temp, mut shared, _backend, _branch) = fixture();
    let alice = shared.identities["alice"].clone();
    let repository = alice.repositories.iter().next().unwrap().clone();
    let mut bob = alice.clone();
    bob.principal = "bob".into();
    let state = Arc::get_mut(&mut shared).unwrap();
    state.identities.insert("alias-alice".into(), alice.clone());
    state.identities.insert("bob".into(), bob);
    shared
        .backend
        .call(
            crate::BackendAction::SetGrants,
            &alice,
            json!({
                "repository_id":repository,"operation_id":new_id("op"),
                "grants":{"read":["alice","bob"],"write":["alice"],"manage":["alice"]}
            }),
        )
        .unwrap();

    let mut idle = Vec::new();
    for _ in 0..MAX_SESSIONS_PER_ACCOUNT {
        let (connection, response) = connect(shared.clone(), "alice", &repository);
        assert!(matches!(response, Response::Ok { .. }));
        idle.push(connection);
    }
    let (rejected, response) = connect(shared.clone(), "alias-alice", &repository);
    assert!(matches!(response, Response::Error { code, .. } if code == "RESOURCE_LIMIT_EXCEEDED"));
    disconnect(rejected, false);
    let (other, response) = connect(shared.clone(), "bob", &repository);
    assert!(matches!(response, Response::Ok { .. }));
    assert_eq!(
        shared.sessions_per_account.lock().unwrap()["alice"],
        MAX_SESSIONS_PER_ACCOUNT
    );
    assert_eq!(shared.sessions_per_account.lock().unwrap()["bob"], 1);

    disconnect(idle.pop().unwrap(), false);
    let (replacement, response) = connect(shared.clone(), "alias-alice", &repository);
    assert!(matches!(response, Response::Ok { .. }));
    idle.push(replacement);
    for connection in idle {
        disconnect(connection, true);
    }
    disconnect(other, true);
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
