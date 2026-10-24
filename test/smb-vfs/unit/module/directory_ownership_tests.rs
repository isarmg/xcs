use super::*;
use std::{cell::Cell, rc::Rc};

struct Open {
    handle: u64,
    directory: bool,
    drops: Rc<Cell<usize>>,
}
impl Drop for Open {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
fn identity(opened: &Open) -> (u64, bool) {
    (opened.handle, opened.directory)
}
fn opened(handle: u64, directory: bool, drops: &Rc<Cell<usize>>) -> Open {
    Open {
        handle,
        directory,
        drops: drops.clone(),
    }
}

#[test]
fn directory_cleanup_transfers_only_its_matching_open_once() {
    let drops = Rc::new(Cell::new(0));
    let mut files = BTreeMap::from([(10, opened(7, true, &drops)), (11, opened(8, true, &drops))]);
    let owner = take_directory_open(&mut files, 10, 7, identity).unwrap();
    assert!(!files.contains_key(&10));
    assert!(files.contains_key(&11));
    assert_eq!(
        drops.get(),
        0,
        "terminal cleanup owns the removed descriptor"
    );
    drop(owner);
    assert_eq!(drops.get(), 1);
    assert!(take_directory_open(&mut files, 10, 7, identity).is_err());
    assert_eq!(drops.get(), 1);
}

#[test]
fn stale_directory_cleanup_preserves_reused_host_slot_and_file_handles() {
    let drops = Rc::new(Cell::new(0));
    let mut files = BTreeMap::from([
        (10, opened(8, true, &drops)),
        (11, opened(7, false, &drops)),
    ]);
    for (slot, handle) in [(10, 7), (11, 7), (12, 7)] {
        assert!(take_directory_open(&mut files, slot, handle, identity).is_err());
        assert_eq!(files.len(), 2);
        assert_eq!(drops.get(), 0);
    }
}

#[test]
fn terminal_cleanup_closes_the_actual_descriptor_even_when_the_remote_close_fails() {
    use smb_protocol::{client::close_descriptor, protocol::Request};
    use std::{
        fs::File,
        io::Read,
        os::fd::{FromRawFd, OwnedFd},
        sync::Mutex,
    };
    struct Resource {
        handle: u64,
        descriptor: OwnedFd,
        io: Mutex<()>,
    }
    fn pipe() -> (File, OwnedFd) {
        let mut descriptors = [-1; 2];
        // SAFETY: pipe2 receives two writable integers and creates only local,
        // nonblocking pipe descriptors. There is no socket or listening endpoint.
        let result =
            unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) };
        assert_eq!(result, 0);
        // SAFETY: Successful pipe2 transferred each distinct descriptor exactly once.
        let observer = unsafe { File::from_raw_fd(descriptors[0]) };
        // SAFETY: The write descriptor is the other sole-owned pipe endpoint.
        let descriptor = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
        (observer, descriptor)
    }
    for remote_failure in [false, true] {
        let (mut observer, descriptor) = pipe();
        let mut files = BTreeMap::from([(
            10,
            Resource {
                handle: 7,
                descriptor,
                io: Mutex::new(()),
            },
        )]);
        let resource = take_directory_open(&mut files, 10, 7, |r| (r.handle, true)).unwrap();
        let result = close_descriptor(
            resource.handle,
            resource.descriptor,
            &resource.io,
            |request| {
                assert!(matches!(request, Request::Close { handle: 7 }));
                if remote_failure {
                    Err(io::ErrorKind::ConnectionAborted.into())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(result.is_err(), remote_failure);
        assert!(
            files.is_empty(),
            "ownership is terminal even when close fails"
        );
        assert_eq!(
            observer.read(&mut [0]).unwrap(),
            0,
            "local descriptor leaked"
        );
        assert!(take_directory_open(&mut files, 10, 7, |r| (r.handle, true)).is_err());
        let (mut replacement, descriptor) = pipe();
        files.insert(
            10,
            Resource {
                handle: 8,
                descriptor,
                io: Mutex::new(()),
            },
        );
        assert!(take_directory_open(&mut files, 10, 7, |r| (r.handle, true)).is_err());
        assert_eq!(
            replacement.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock,
            "stale cleanup must not close a new descriptor occupying the host slot"
        );
        drop(files);
        assert_eq!(replacement.read(&mut [0]).unwrap(), 0);
    }
}
