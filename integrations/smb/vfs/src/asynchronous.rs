//! Own buffers and descriptors in workers; only the host event loop touches
//! talloc, requests, file structures or client-provided output buffers.
use super::{error_errno, sdk};
use std::{
    ffi::c_void,
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{Arc, Mutex, OnceLock, mpsc},
};

pub(super) const MAX_IO: usize = 8 * 1024 * 1024;

pub(super) struct Completion {
    pub result: isize,
    pub bytes: Vec<u8>,
}
impl Completion {
    pub fn done(result: isize) -> Self {
        Self {
            result,
            bytes: Vec::new(),
        }
    }
}
struct Outcome {
    error: i32,
    completion: Completion,
}
type Shared = Arc<Mutex<Option<Outcome>>>;
type Job = Box<dyn FnOnce() -> io::Result<Completion> + Send>;
struct Work {
    job: Job,
    shared: Shared,
    wake: OwnedFd,
}

fn pool() -> io::Result<&'static mpsc::SyncSender<Work>> {
    static POOL: OnceLock<Option<mpsc::SyncSender<Work>>> = OnceLock::new();
    POOL.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<Work>(8);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut started = 0;
        for index in 0..2 {
            let receiver = receiver.clone();
            if std::thread::Builder::new()
                .name(format!("zxs-smb-io-{index}"))
                .spawn(move || {
                    loop {
                        let work = match receiver.lock() {
                            Ok(receiver) => match receiver.recv() {
                                Ok(work) => work,
                                Err(_) => return,
                            },
                            Err(_) => return,
                        };
                        let result =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(work.job))
                                .unwrap_or_else(|_| Err(io::Error::from_raw_os_error(libc::EIO)));
                        let outcome = match result {
                            Ok(completion) => Outcome {
                                error: 0,
                                completion,
                            },
                            Err(error) => {
                                eprintln!("zxs SMB async: {error}");
                                Outcome {
                                    error: error_errno(&error),
                                    completion: Completion::done(-1),
                                }
                            }
                        };
                        if let Ok(mut slot) = work.shared.lock() {
                            *slot = Some(outcome);
                        }
                        let signal = 1_u64;
                        loop {
                            let result = unsafe {
                                libc::write(
                                    work.wake.as_raw_fd(),
                                    (&signal as *const u64).cast(),
                                    8,
                                )
                            };
                            if result >= 0
                                || io::Error::last_os_error().raw_os_error() != Some(libc::EINTR)
                            {
                                break;
                            }
                        }
                    }
                })
                .is_ok()
            {
                started += 1;
            }
        }
        (started > 0).then_some(sender)
    })
    .as_ref()
    .ok_or_else(|| io::Error::from_raw_os_error(libc::EAGAIN))
}

#[repr(C)]
struct State {
    extra: *mut Extra,
    result: isize,
    error: i32,
}
struct Extra {
    request: *mut sdk::tevent_req,
    read_fd: i32,
    shared: Shared,
    destination: *mut c_void,
    capacity: usize,
}
unsafe extern "C" fn destroy(pointer: *mut c_void) -> i32 {
    let state = pointer.cast::<State>();
    let extra = unsafe { (*state).extra };
    if !extra.is_null() {
        unsafe {
            (*state).extra = std::ptr::null_mut();
            drop(Box::from_raw(extra));
        }
    }
    0
}
unsafe extern "C" fn completed(
    _event: *mut sdk::tevent_context,
    fd: *mut sdk::tevent_fd,
    _flags: u16,
    pointer: *mut c_void,
) {
    let state = pointer.cast::<State>();
    let extra = unsafe { &*(*state).extra };
    let mut signal = 0_u64;
    unsafe {
        libc::read(extra.read_fd, (&mut signal as *mut u64).cast(), 8);
    }
    let outcome = extra
        .shared
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
        .unwrap_or(Outcome {
            error: libc::EIO,
            completion: Completion::done(-1),
        });
    let request = extra.request;
    let mut error = outcome.error;
    if !outcome.completion.bytes.is_empty() {
        if extra.destination.is_null() || outcome.completion.bytes.len() > extra.capacity {
            error = libc::EINVAL;
        } else {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    outcome.completion.bytes.as_ptr(),
                    extra.destination.cast::<u8>(),
                    outcome.completion.bytes.len(),
                );
            }
        }
    }
    unsafe {
        (*state).result = if error == 0 {
            outcome.completion.result
        } else {
            -1
        };
        (*state).error = error;
        // Unregister before closing the watched descriptor, as tevent requires.
        sdk::_talloc_free(fd.cast(), c"zxs Rust VFS completion".as_ptr());
        sdk::_tevent_req_done(request, c"zxs Rust VFS completion".as_ptr());
    }
}

pub(super) unsafe fn send(
    mem_ctx: *mut sdk::TALLOC_CTX,
    event: *mut sdk::tevent_context,
    destination: *mut c_void,
    capacity: usize,
    job: impl FnOnce() -> io::Result<Completion> + Send + 'static,
) -> *mut sdk::tevent_req {
    let mut state: *mut State = std::ptr::null_mut();
    let request = unsafe {
        sdk::__tevent_req_create(
            mem_ctx,
            (&mut state as *mut *mut State).cast(),
            std::mem::size_of::<State>(),
            c"zxs_async_state".as_ptr(),
            c"zxs_async_send".as_ptr(),
            c"zxs Rust VFS".as_ptr(),
        )
    };
    if request.is_null() {
        super::errno(libc::ENOMEM);
        return request;
    }
    let shared = Arc::new(Mutex::new(None));
    unsafe {
        (*state).result = -1;
        (*state).error = libc::EINPROGRESS;
        (*state).extra = Box::into_raw(Box::new(Extra {
            request,
            read_fd: -1,
            shared: shared.clone(),
            destination,
            capacity,
        }));
        sdk::_talloc_set_destructor(state.cast(), Some(destroy));
    }
    let mut watcher: *mut sdk::tevent_fd = std::ptr::null_mut();
    let prepared = (|| -> io::Result<()> {
        let descriptor = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        let read = unsafe { OwnedFd::from_raw_fd(descriptor) };
        let duplicate = unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(io::Error::last_os_error());
        }
        let wake = unsafe { OwnedFd::from_raw_fd(duplicate) };
        watcher = unsafe {
            sdk::_tevent_add_fd(
                event,
                state.cast(),
                descriptor,
                sdk::TEVENT_FD_READ as u16,
                Some(completed),
                state.cast(),
                c"zxs_async_completed".as_ptr(),
                c"zxs Rust VFS".as_ptr(),
            )
        };
        if watcher.is_null() {
            return Err(io::Error::from_raw_os_error(libc::ENOMEM));
        }
        unsafe {
            (*(*state).extra).read_fd = descriptor;
            sdk::tevent_fd_set_auto_close(watcher);
        }
        std::mem::forget(read); // The tevent watcher now owns this descriptor.
        pool()?
            .try_send(Work {
                job: Box::new(job),
                shared,
                wake,
            })
            .map_err(|_| io::Error::from_raw_os_error(libc::EAGAIN))?;
        Ok(())
    })();
    if let Err(error) = prepared {
        unsafe {
            if !watcher.is_null() {
                sdk::_talloc_free(watcher.cast(), c"zxs Rust VFS queue failure".as_ptr());
            }
            (*state).error = error_errno(&error);
            sdk::_tevent_req_done(request, c"zxs Rust VFS queue failure".as_ptr());
            return sdk::tevent_req_post(request, event);
        }
    }
    request
}

pub(super) unsafe fn recv(request: *mut sdk::tevent_req, aio: *mut sdk::vfs_aio_state) -> isize {
    if request.is_null() || aio.is_null() {
        super::errno(libc::EINVAL);
        return -1;
    }
    let state = unsafe { sdk::_tevent_req_data(request).cast::<State>() };
    let (result, error) = unsafe { ((*state).result, (*state).error) };
    unsafe {
        (*aio).error = error;
        (*aio).duration = 0;
        sdk::tevent_req_received(request);
    }
    if error != 0 {
        super::errno(error);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
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
        unsafe {
            let event = sdk::tevent_context_init(std::ptr::null_mut());
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
    fn cancelling_request_before_worker_finishes_does_not_touch_released_host_state() {
        let (started, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let mut destination = [0_u8; 4];
        unsafe {
            let event = sdk::tevent_context_init(std::ptr::null_mut());
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
}
