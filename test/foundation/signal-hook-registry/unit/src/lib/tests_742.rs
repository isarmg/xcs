#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[cfg(not(windows))]
    use libc::{pid_t, SIGUSR1, SIGUSR2};

    #[cfg(windows)]
    use libc::SIGTERM as SIGUSR1;
    #[cfg(windows)]
    use libc::SIGTERM as SIGUSR2;

    use super::*;

    #[test]
    #[should_panic]
    fn panic_forbidden() {
        let _ = unsafe { register(SIGILL, || ()) };
    }

    /// Registering the forbidden signals is allowed in the _unchecked version.
    #[test]
    #[allow(clippy::redundant_closure)] // Clippy, you're wrong. Because it changes the return value.
    fn forbidden_raw() {
        unsafe { register_signal_unchecked(SIGFPE, || std::process::abort()).unwrap() };
    }

    #[test]
    fn signal_without_pid() {
        let status = Arc::new(AtomicUsize::new(0));
        let action = {
            let status = Arc::clone(&status);
            move || {
                status.store(1, Ordering::Relaxed);
            }
        };
        unsafe {
            register(SIGUSR2, action).unwrap();
            libc::raise(SIGUSR2);
        }
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(100));
            let current = status.load(Ordering::Relaxed);
            match current {
                // Not yet
                0 => continue,
                // Good, we are done with the correct result
                _ if current == 1 => return,
                _ => panic!("Wrong result value {}", current),
            }
        }
        panic!("Timed out waiting for the signal");
    }

    #[test]
    #[cfg(not(windows))]
    fn signal_with_pid() {
        let status = Arc::new(AtomicUsize::new(0));
        let action = {
            let status = Arc::clone(&status);
            move |siginfo: &siginfo_t| {
                // Hack: currently, libc exposes only the first 3 fields of siginfo_t. The pid
                // comes somewhat later on. Therefore, we do a Really Ugly Hack and define our
                // own structure (and hope it is correct on all platforms). But hey, this is
                // only the tests, so we are going to get away with this.
                #[repr(C)]
                struct SigInfo {
                    _fields: [c_int; 3],
                    #[cfg(all(target_pointer_width = "64", target_os = "linux"))]
                    _pad: c_int,
                    pid: pid_t,
                }
                let s: &SigInfo = unsafe {
                    (siginfo as *const _ as usize as *const SigInfo)
                        .as_ref()
                        .unwrap()
                };
                status.store(s.pid as usize, Ordering::Relaxed);
            }
        };
        let pid;
        unsafe {
            pid = libc::getpid();
            register_sigaction(SIGUSR2, action).unwrap();
            libc::raise(SIGUSR2);
        }
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(100));
            let current = status.load(Ordering::Relaxed);
            match current {
                // Not yet (PID == 0 doesn't happen)
                0 => continue,
                // Good, we are done with the correct result
                _ if current == pid as usize => return,
                _ => panic!("Wrong status value {}", current),
            }
        }
        panic!("Timed out waiting for the signal");
    }

    /// Check that registration works as expected and that unregister tells if it did or not.
    #[test]
    fn register_unregister() {
        let signal = unsafe { register(SIGUSR1, || ()).unwrap() };
        // It was there now, so we can unregister
        assert!(unregister(signal));
        // The next time unregistering does nothing and tells us so.
        assert!(!unregister(signal));
    }

    /// Check that errno is not clobbered by the signal handler.
    #[test]
    fn save_restore_errno() {
        const MAGIC_ERRNO: i32 = 123456;
        let action = move || {
            errno::set_errno(Errno(MAGIC_ERRNO));
        };
        unsafe {
            register(SIGUSR1, action).unwrap();
            libc::raise(SIGUSR1);
        }
        // NB: raise() might clobber errno on some platforms, so this test isn't waterproof. But it
        // fails at least sometimes on some platforms if the errno save/restore is removed.
        assert!(errno::errno().0 != MAGIC_ERRNO);
    }
}
