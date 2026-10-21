use super::*;

#[test]
fn input_buffer_checks_null_empty_and_oversize_before_borrowing() {
    // SAFETY: null/zero and invalid sizes must be rejected before dereference;
    // the only nonempty valid buffer is a live local array borrowed here.
    unsafe {
        assert!(input_bytes(std::ptr::null(), 0).unwrap().is_empty());
        assert!(input_bytes(std::ptr::null(), 1).is_err());
        let bytes = [1_u8, 2, 3];
        assert!(input_bytes(bytes.as_ptr().cast(), usize::MAX).is_err());
        assert_eq!(
            input_bytes(bytes.as_ptr().cast(), bytes.len()).unwrap(),
            &bytes
        );
    }
}

#[test]
fn removed_open_descriptor_is_closed_on_poisoned_lock_and_remote_save_failure() {
    use std::{io::Read, os::unix::net::UnixStream};
    for poisoned in [true, false] {
        let (descriptor, mut peer) = UnixStream::pair().unwrap();
        peer.set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();
        let lock = Arc::new(Mutex::new(()));
        if poisoned {
            let _ = std::panic::catch_unwind(|| {
                let _guard = lock.lock().unwrap();
                panic!("simulate a failed I/O worker");
            });
        }
        let opened = Open {
            info: OpenResult {
                entry_id: "entry".into(),
                kind: "file".into(),
                path: "file.txt".into(),
                handle: 7,
                size: 0,
                max_file_bytes: 1,
                version_id: None,
                workspace_id: None,
            },
            fd: descriptor.into_raw_fd(),
            writable: true,
            io: lock,
        };
        let error = close_opened(opened, |request| {
            if poisoned {
                assert!(matches!(request, Request::Detach { handle: 7 }));
                Ok(())
            } else {
                assert!(matches!(request, Request::Close { handle: 7 }));
                Err(std::io::ErrorKind::PermissionDenied.into())
            }
        })
        .unwrap_err();
        if poisoned {
            assert_eq!(error.raw_os_error(), Some(libc::EIO));
        } else {
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        }
        assert_eq!(
            peer.read(&mut [0; 1]).unwrap(),
            0,
            "removed descriptor leaked"
        );
    }
}
#[test]
fn failed_name_lookup_preserves_the_callers_filename() {
    let original = c"new.txt".as_ptr().cast_mut();
    let mut found = original;
    // SAFETY: The output points to a live local slot and the static invalid component is
    // rejected before any null host handle is accessed.
    let status = unsafe {
        real_filename(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            c"..".as_ptr(),
            std::ptr::null_mut(),
            &mut found,
        )
    };
    assert_ne!(status.v, 0);
    assert_eq!(found, original);
}
#[test]
fn absolute_names_are_confined_to_the_exact_share_root() {
    let root = c"/safe/share";
    let mut connection = sdk::connection_struct {
        connectpath: root.as_ptr().cast_mut(),
        ..Default::default()
    };
    let mut handle = sdk::vfs_handle_struct {
        conn: &mut connection,
        ..Default::default()
    };
    let mut file: sdk::smb_filename = Default::default();
    for (input, expected) in [
        ("/safe/share", Some("")),
        ("/safe/share/./文件.txt", Some("文件.txt")),
        ("./documents/file.txt", Some("documents/file.txt")),
        ("/safe/share-other/file", None),
        ("/safe/share/../secret", None),
        ("/outside/file", None),
        ("documents/../../secret", None),
    ] {
        let input = std::ffi::CString::new(input).unwrap();
        file.base_name = input.as_ptr().cast_mut();
        // SAFETY: All SDK objects and CString storage are local, initialized, and live until
        // name returns; their pointers reference those objects.
        let result = unsafe { name(&mut handle, &file) };
        match expected {
            Some(expected) => assert_eq!(result.unwrap(), expected),
            None => assert!(result.is_err()),
        }
    }
}
