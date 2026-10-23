//! Snapshot, ownership and live-authorization checks over the actual local RPC.
use super::binding_tests::{exchange, wire};
use super::*;
use smb_protocol::protocol::DirectoryPage;

fn page(wire: &mut FrameStream, handle: u64, restart: bool) -> DirectoryPage {
    let (reply, fd) = exchange(wire, Request::ReadDirectory { handle, restart });
    assert!(fd.is_none());
    let Response::Ok { result } = reply else {
        panic!("directory read failed: {reply:?}")
    };
    json_codec::from_value(result).unwrap()
}
fn root(wire: &mut FrameStream) -> u64 {
    let (reply, descriptor) = exchange(
        wire,
        Request::Open {
            path: String::new(),
            writable: false,
            create: false,
            truncate: false,
            exclusive: false,
            write_through: false,
        },
    );
    assert!(descriptor.is_some());
    let Response::Ok { result } = reply else {
        panic!("root open failed")
    };
    json_codec::from_value::<OpenResult>(result).unwrap().handle
}
#[test]
fn directory_cursor_freezes_its_commit_and_restart_captures_a_new_snapshot() {
    let (_temp, shared, _backend, branch) = fixture();
    let (mut client, worker, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Ok { .. }));
    let handle = root(&mut client);
    let first = page(&mut client, handle, false);
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.entries[0].path, "report.txt");
    let mut writer = session(shared.clone(), &branch);
    writer
        .namespace(
            "mkdir".into(),
            "new-directory".into(),
            None,
            false,
            &new_id("op"),
        )
        .unwrap();
    assert!(page(&mut client, handle, false).entries.is_empty());
    let restarted = page(&mut client, handle, true);
    assert_eq!(
        restarted
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        vec!["new-directory", "report.txt"]
    );
    assert!(!restarted.more);
    assert!(matches!(
        exchange(
            &mut client,
            Request::Stat {
                path: "report.txt".into()
            }
        )
        .0,
        Response::Ok { .. }
    ));
    assert!(matches!(
        exchange(&mut client, Request::Close { handle }).0,
        Response::Ok { .. }
    ));
    drop(client);
    worker.join().unwrap().unwrap();
}
#[test]
fn directory_pages_recheck_current_account_permissions_and_handle_ownership() {
    let (_temp, shared, _backend, _branch) = fixture();
    let (mut client, worker, hello) = wire(shared.clone());
    assert!(matches!(hello, Response::Ok { .. }));
    let handle = root(&mut client);
    assert_eq!(page(&mut client, handle, false).entries.len(), 1);
    let mut administrator = shared.identities["alice"].clone();
    administrator.admin = true;
    let repository = administrator.repositories.iter().next().unwrap().clone();
    shared
        .backend
        .call(
            crate::BackendAction::SetGrants,
            &administrator,
            json!({
                "repository_id":repository,"operation_id":new_id("op"),
                "grants":{"read":[],"write":[],"manage":["replacement-manager"]},
            }),
        )
        .unwrap();
    assert!(
        matches!(exchange(&mut client,Request::ReadDirectory {handle,restart:false}).0,Response::Error {code,..} if code=="FORBIDDEN")
    );
    assert!(
        matches!(exchange(&mut client,Request::ReadDirectory {handle:handle+100,restart:false}).0,Response::Error {code,..} if code=="NOT_FOUND")
    );
    assert!(matches!(
        exchange(&mut client, Request::Close { handle }).0,
        Response::Ok { .. }
    ));
    drop(client);
    worker.join().unwrap().unwrap();
}
#[test]
fn renamed_and_replaced_directory_does_not_retarget_the_open_handle() {
    let (_temp, shared, _backend, branch) = fixture();
    let mut session = session(shared, &branch);
    session
        .namespace(
            "mkdir".into(),
            "original".into(),
            None,
            false,
            &new_id("op"),
        )
        .unwrap();
    let (opened, _) = session
        .open("original".into(), false, false, false, false, false)
        .unwrap();
    let opened: OpenResult = json_codec::from_value(opened).unwrap();
    session
        .namespace(
            "rename".into(),
            "original".into(),
            Some("moved".into()),
            false,
            &new_id("op"),
        )
        .unwrap();
    session
        .namespace(
            "mkdir".into(),
            "original".into(),
            None,
            false,
            &new_id("op"),
        )
        .unwrap();
    // The open descriptor keeps its snapshot, even before its first read.
    // Rewind cannot follow the replacement at the original alias path.
    let (old, _) = session
        .dispatch(Request::ReadDirectory {
            handle: opened.handle,
            restart: false,
        })
        .unwrap();
    assert!(
        json_codec::from_value::<DirectoryPage>(old)
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        session
            .dispatch(Request::ReadDirectory {
                handle: opened.handle,
                restart: true,
            })
            .unwrap_err()
            .code,
        "SMB_REBIND_REQUIRED"
    );
}
