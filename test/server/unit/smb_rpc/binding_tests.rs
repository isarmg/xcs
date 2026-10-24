//! Real core and Unix RPC regression for live SMB binding admission.
use super::*;
use std::{os::unix::net::UnixStream, thread::JoinHandle};

pub(super) fn wire(shared: Arc<Shared>) -> (FrameStream, JoinHandle<io::Result<()>>, Response) {
    let repository = repository(&shared);
    let (client, server) = UnixStream::pair().unwrap();
    let worker =
        std::thread::spawn(move || run_connection(FrameStream::new(server).unwrap(), shared));
    let mut stream = FrameStream::new(client).unwrap();
    stream
        .send(
            &Request::Hello {
                protocol: PROTOCOL.into(),
                capabilities: REQUIRED_CAPABILITIES
                    .iter()
                    .map(|value| (*value).into())
                    .collect(),
                username: "alice".into(),
                repository_id: repository,
            },
            None,
        )
        .unwrap();
    let (response, descriptor) = stream.receive::<Response>().unwrap();
    assert!(descriptor.is_none());
    (stream, worker, response)
}
pub(super) fn exchange(wire: &mut FrameStream, request: Request) -> (Response, Option<OwnedFd>) {
    wire.send(&request, None).unwrap();
    wire.receive().unwrap()
}
pub(super) fn rebind(shared: &Shared, branch: &str, enabled: bool) {
    let administrator = identity(shared);
    let repository = repository(shared);
    let binding = shared
        .backend
        .call(
            crate::BackendAction::SmbBinding,
            &administrator,
            json!({"repository_id":repository}),
        )
        .unwrap();
    shared.backend.call(crate::BackendAction::UpdateSmbBinding, &administrator, json!({
        "repository_id":repository,"branch_id":branch,"enabled":enabled,
        "expected_history_generation":binding["history_generation"],"operation_id":new_id("op"),
    })).unwrap();
}
fn refused(wire: &mut FrameStream, request: Request) {
    let (response, fd) = exchange(wire, request);
    assert!(fd.is_none());
    assert!(matches!(response, Response::Error { code, .. } if code == "SMB_REBIND_REQUIRED"));
}
fn create(path: &str) -> Request {
    Request::Open {
        path: path.into(),
        writable: true,
        create: true,
        truncate: false,
        exclusive: true,
        write_through: false,
    }
}
#[test]
fn disabled_and_reenabled_binding_cannot_revive_an_idle_authenticated_connection() {
    let (_temp, shared, _backend, branch) = fixture();
    let (mut old, worker, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Ok { .. }));
    rebind(&shared, &branch, false);
    refused(&mut old, create("after-disable.txt"));
    refused(
        &mut old,
        Request::Stat {
            path: String::new(),
        },
    );
    refused(
        &mut old,
        Request::Namespace {
            operation_id: new_id("op"),
            kind: "mkdir".into(),
            path: "disabled".into(),
            destination: None,
            replace: false,
        },
    );
    let (new, rejected, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Error { code, .. } if code == "SMB_REBIND_REQUIRED"));
    drop(new);
    rejected.join().unwrap().unwrap();
    rebind(&shared, &branch, true);
    refused(&mut old, create("after-reenable.txt"));
    let (mut new, admitted, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Ok { .. }));
    assert!(matches!(
        exchange(
            &mut new,
            Request::Stat {
                path: "report.txt".into()
            }
        )
        .0,
        Response::Ok { .. }
    ));
    let (opened, fd) = exchange(&mut new, create("fresh.txt"));
    let Response::Ok { result } = opened else {
        panic!("fresh binding must admit write")
    };
    let opened: OpenResult = json_codec::from_value(result).unwrap();
    assert!(fd.is_some());
    assert!(matches!(
        exchange(
            &mut new,
            Request::Close {
                handle: opened.handle
            }
        )
        .0,
        Response::Ok { .. }
    ));
    drop(fd);
    drop(new);
    admitted.join().unwrap().unwrap();
    drop(old);
    worker.join().unwrap().unwrap();
}

#[test]
fn existing_reader_keeps_its_version_on_normal_writes_but_stops_after_binding_change() {
    let (_temp, shared, _backend, branch) = fixture();
    let (mut wire, worker, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Ok { .. }));
    let (opened, fd) = exchange(
        &mut wire,
        Request::Open {
            path: "report.txt".into(),
            writable: false,
            create: false,
            truncate: false,
            exclusive: false,
            write_through: false,
        },
    );
    let Response::Ok { result } = opened else {
        panic!("read open failed")
    };
    let opened: OpenResult = json_codec::from_value(result).unwrap();
    let read = File::from(fd.unwrap());
    let mut writer = session(shared.clone(), &branch);
    let (write, bytes) = open_file(&mut writer, true);
    bytes.write_all_at(b"new", 0).unwrap();
    writer.dirty(write.handle).unwrap();
    writer.close(write.handle, false).unwrap();
    assert!(matches!(
        exchange(
            &mut wire,
            Request::AuthorizeRead {
                handle: opened.handle
            }
        )
        .0,
        Response::Ok { .. }
    ));
    let mut old = [0; 3];
    read.read_exact_at(&mut old, 0).unwrap();
    assert_eq!(&old, b"old");
    rebind(&shared, &branch, true);
    refused(
        &mut wire,
        Request::AuthorizeRead {
            handle: opened.handle,
        },
    );
    refused(&mut wire, create("after-rebind.txt"));
    assert!(matches!(
        exchange(
            &mut wire,
            Request::Close {
                handle: opened.handle
            }
        )
        .0,
        Response::Ok { .. }
    ));
    drop(read);
    drop(wire);
    worker.join().unwrap().unwrap();
}

#[test]
fn live_writer_cannot_prepare_or_publish_after_disable_and_failed_close_retains_bytes() {
    let (_temp, shared, backend, branch) = fixture();
    let (mut wire, worker, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Ok { .. }));
    let (opened, fd) = exchange(
        &mut wire,
        Request::Open {
            path: "report.txt".into(),
            writable: true,
            create: false,
            truncate: false,
            exclusive: false,
            write_through: false,
        },
    );
    let Response::Ok { result } = opened else {
        panic!("write open failed")
    };
    let opened: OpenResult = json_codec::from_value(result).unwrap();
    let file = File::from(fd.unwrap());
    assert!(matches!(
        exchange(
            &mut wire,
            Request::PrepareWrite {
                handle: opened.handle,
                end: 3,
                bytes: 3
            }
        )
        .0,
        Response::Ok { .. }
    ));
    file.write_all_at(b"new", 0).unwrap();
    assert!(matches!(
        exchange(
            &mut wire,
            Request::Dirty {
                handle: opened.handle
            }
        )
        .0,
        Response::Ok { .. }
    ));
    // Active workspaces correctly block administrative rebind. A published
    // writer remains open without an active workspace, so disable is allowed.
    let administrator = identity(&shared);
    let repository = repository(&shared);
    let binding = shared
        .backend
        .call(
            crate::BackendAction::SmbBinding,
            &administrator,
            json!({"repository_id":repository}),
        )
        .unwrap();
    assert_eq!(shared.backend.call(crate::BackendAction::UpdateSmbBinding, &administrator, json!({
        "repository_id":repository,"branch_id":branch,"enabled":false,
        "expected_history_generation":binding["history_generation"],"operation_id":new_id("op"),
    })).unwrap_err().code, "WORKSPACE_BUSY");
    assert!(matches!(
        exchange(
            &mut wire,
            Request::Flush {
                handle: opened.handle
            }
        )
        .0,
        Response::Ok { .. }
    ));
    rebind(&shared, &branch, false);
    // Model bytes from a previously admitted/in-flight FD write. Revocation
    // does not forcibly close a descriptor held by the trusted host process.
    file.write_all_at(b"bad", 0).unwrap();
    for request in [
        Request::PrepareWrite {
            handle: opened.handle,
            end: 4,
            bytes: 1,
        },
        Request::Dirty {
            handle: opened.handle,
        },
    ] {
        refused(&mut wire, request);
    }
    for request in [
        Request::Flush {
            handle: opened.handle,
        },
        Request::Close {
            handle: opened.handle,
        },
    ] {
        let (reply, fd) = exchange(&mut wire, request);
        assert!(fd.is_none());
        assert!(
            matches!(reply, Response::Error { code, .. } if code == "WORKSPACE_REBASE_REQUIRED")
        );
    }
    assert_eq!(backend.uploads.load(Ordering::SeqCst), 1);
    assert!(shared.file_writers.lock().unwrap().is_empty());
    assert!(
        matches!(exchange(&mut wire, Request::AuthorizeRead { handle: opened.handle }).0, Response::Error { code, .. } if code == "NOT_FOUND")
    );
    let records: Vec<_> = fs::read_dir(&shared.spool)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".work.json"))
        .collect();
    assert_eq!(records.len(), 1);
    let record: Value = json_codec::from_slice(&fs::read(&records[0]).unwrap()).unwrap();
    assert_eq!(record["handle_state"], "detached");
    assert_eq!(record["detach_error"]["code"], "WORKSPACE_REBASE_REQUIRED");
    assert_eq!(
        fs::read(record["source_path"].as_str().unwrap()).unwrap(),
        b"bad"
    );
    drop(file);
    drop(wire);
    worker.join().unwrap().unwrap();
}
