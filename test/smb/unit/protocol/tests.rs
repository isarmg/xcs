use super::*;
use std::{fs::File, io::Seek, os::fd::AsFd};

#[test]
fn namespace_requests_require_a_preallocated_identity_on_the_wire() {
    for kind in ["mkdir", "rename", "unlink", "rmdir"] {
        let request = namespace_request(kind, "entry".into(), None, false);
        let value = serde_json::to_value(&request).unwrap();
        storage_contract::validate_id(value["operation_id"].as_str().unwrap(), "op").unwrap();
        let mut missing = value;
        missing.as_object_mut().unwrap().remove("operation_id");
        assert!(serde_json::from_value::<Request>(missing).is_err());
    }
}

#[test]
fn unlinkat_flags_preserve_target_kind_and_reject_unknown_flags() {
    for (flags, expected) in [(0, "unlink"), (libc::AT_REMOVEDIR, "rmdir")] {
        let request = remove_request("entry".into(), flags).unwrap();
        let Request::Namespace {
            operation_id,
            kind,
            path,
            destination,
            replace,
        } = request
        else {
            panic!("expected namespace request");
        };
        storage_contract::validate_id(&operation_id, "op").unwrap();
        assert_eq!(kind, expected);
        assert_eq!(path, "entry");
        assert!(destination.is_none());
        assert!(!replace);
    }
    for flags in [-1, libc::AT_SYMLINK_NOFOLLOW, libc::AT_REMOVEDIR | 1] {
        assert_eq!(
            remove_request("entry".into(), flags)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL)
        );
    }
}

#[test]
fn framing_transfers_an_owned_cloexec_descriptor_without_a_path() {
    let (one, two) = UnixStream::pair().unwrap();
    let mut sender = FrameStream::new(one).unwrap();
    let mut receiver = FrameStream::new(two).unwrap();
    let source = File::open("/etc/hostname").unwrap();
    sender
        .send(
            &Response::Ok {
                result: serde_json::json!({"handle":"18446744073709551615"}),
            },
            Some(source.as_fd()),
        )
        .unwrap();
    let (result, fd) = receiver.receive::<Response>().unwrap();
    assert!(matches!(result, Response::Ok { .. }));
    let fd = fd.unwrap();
    // SAFETY: F_GETFD only reads the flag word of this live owned descriptor.
    assert_ne!(
        unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    let mut transferred = File::from(fd);
    transferred.rewind().unwrap();
    assert!(transferred.metadata().unwrap().is_file());
}

#[test]
fn oversized_length_is_rejected_before_allocating_payload() {
    let (mut one, two) = UnixStream::pair().unwrap();
    one.write_all(&u32::MAX.to_be_bytes()).unwrap();
    let error = FrameStream::new(two)
        .unwrap()
        .receive::<Request>()
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}
