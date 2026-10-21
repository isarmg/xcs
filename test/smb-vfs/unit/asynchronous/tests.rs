use super::*;
use std::time::Duration;

// tevent.c initializes a process-global backend list without synchronization.
// Samba initializes it on the host event loop; Rust's parallel test threads
// must not race the first backend registration. Only context creation is
// serialized: independent request execution and cancellation stay concurrent.
static EVENT_INITIALIZATION: Mutex<()> = Mutex::new(());

fn event_context() -> *mut sdk::tevent_context {
    let _initialization = EVENT_INITIALIZATION.lock().unwrap();
    // SAFETY: a null talloc parent requests a new independently owned event
    // context; the global backend initialization is serialized above.
    unsafe { sdk::tevent_context_init(std::ptr::null_mut()) }
}
#[link(name = "tevent-private-samba")]
#[link(name = "talloc-private-samba")]
#[link(name = "smbconf")]
unsafe extern "C" {}

// Registration is the only host stub. Event and allocation functions come
// from the actual libraries built with the configured target SDK.
#[unsafe(export_name = "smb_register_vfs")]
unsafe extern "C" fn register(
    _version: i32,
    _name: *const libc::c_char,
    _table: *const sdk::vfs_fn_pointers,
) -> sdk::NTSTATUS {
    super::super::nt_status(0)
}
#[test]
fn worker_wait_keeps_host_event_loop_pending_and_read_is_copied_on_completion() {
    let (started, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut bytes = [0_u8; 4];
    // SAFETY: This test uses real SDK allocations on one thread and live local output buffers.
    // Requests are completed or freed before output/event storage is released; each allocation
    // is freed once.
    unsafe {
        let event = event_context();
        assert!(!event.is_null());
        let request = send(
            std::ptr::null_mut(),
            event,
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            move || {
                started.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(5))
                    .map_err(io::Error::other)?;
                Ok(Completion {
                    result: 4,
                    bytes: vec![1, 2, 3, 4],
                })
            },
        );
        assert!(!request.is_null());
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(sdk::tevent_req_is_in_progress(request));
        assert_eq!(bytes, [0; 4]);
        release.send(()).unwrap();
        while sdk::tevent_req_is_in_progress(request) {
            assert_eq!(sdk::_tevent_loop_once(event, c"zxs SDK test".as_ptr()), 0);
        }
        let mut aio: sdk::vfs_aio_state = std::mem::zeroed();
        assert_eq!(recv(request, &mut aio), 4);
        assert_eq!(aio.error, 0);
        assert_eq!(bytes, [1, 2, 3, 4]);
        sdk::_talloc_free(request.cast(), c"zxs SDK test".as_ptr());
        sdk::_talloc_free(event.cast(), c"zxs SDK test".as_ptr());
    }
}
#[test]
fn oversized_completion_is_rejected_without_writing_host_buffer() {
    let mut destination = [7_u8; 2];
    // SAFETY: Real SDK allocations and the local output buffer stay live on
    // this thread until completion; request and event are each freed once.
    unsafe {
        let event = event_context();
        assert!(!event.is_null());
        let request = send(
            std::ptr::null_mut(),
            event,
            destination.as_mut_ptr().cast(),
            destination.len(),
            || {
                Ok(Completion {
                    result: 3,
                    bytes: vec![9; 3],
                })
            },
        );
        assert!(!request.is_null());
        while sdk::tevent_req_is_in_progress(request) {
            assert_eq!(
                sdk::_tevent_loop_once(event, c"zxs SDK bounds test".as_ptr()),
                0
            );
        }
        let mut aio: sdk::vfs_aio_state = Default::default();
        assert_eq!(recv(request, &mut aio), -1);
        assert_eq!(aio.error, libc::EINVAL);
        assert_eq!(destination, [7; 2]);
        sdk::_talloc_free(request.cast(), c"zxs SDK bounds test".as_ptr());
        sdk::_talloc_free(event.cast(), c"zxs SDK bounds test".as_ptr());
    }
}

#[test]
fn cancelling_request_before_worker_finishes_does_not_touch_released_host_state() {
    let (started, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    let mut destination = [0_u8; 4];
    // SAFETY: This test uses real SDK allocations on one thread and live local output buffers.
    // Requests are completed or freed before output/event storage is released; each allocation
    // is freed once.
    unsafe {
        let event = event_context();
        assert!(!event.is_null());
        let request = send(
            std::ptr::null_mut(),
            event,
            destination.as_mut_ptr().cast(),
            4,
            move || {
                started.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(5))
                    .map_err(io::Error::other)?;
                finished.send(()).unwrap();
                Ok(Completion {
                    result: 4,
                    bytes: vec![9; 4],
                })
            },
        );
        assert!(!request.is_null());
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        sdk::_talloc_free(request.cast(), c"zxs SDK cancellation test".as_ptr());
        sdk::_talloc_free(event.cast(), c"zxs SDK cancellation test".as_ptr());
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(destination, [0; 4]);
    }
}
