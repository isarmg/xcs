#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(linux_raw)]
    use crate::runtime::{KERNEL_SIGRTMAX, KERNEL_SIGRTMIN};
    use core::mem::{align_of, size_of};

    #[test]
    fn test_assumptions() {
        #[cfg(linux_raw)]
        assert!(KERNEL_SIGRTMAX as usize - 1 < size_of::<KernelSigSet>() * 8);
    }

    #[test]
    fn test_layouts() {
        assert!(size_of::<KernelSigSet>() <= size_of::<libc::sigset_t>());
        assert!(align_of::<KernelSigSet>() <= align_of::<libc::sigset_t>());
    }

    /// A bunch of signals for testing.
    fn sigs() -> Vec<Signal> {
        #[allow(unused_mut)]
        let mut sigs = vec![
            Signal::HUP,
            Signal::INT,
            Signal::QUIT,
            Signal::ILL,
            Signal::TRAP,
            Signal::ABORT,
            Signal::BUS,
            Signal::FPE,
            Signal::KILL,
            Signal::USR1,
            Signal::SEGV,
            Signal::USR2,
            Signal::PIPE,
            Signal::ALARM,
            Signal::TERM,
            Signal::CHILD,
            Signal::CONT,
            Signal::STOP,
            Signal::TSTP,
            Signal::TTIN,
            Signal::TTOU,
            Signal::URG,
            Signal::XCPU,
            Signal::XFSZ,
            Signal::VTALARM,
            Signal::PROF,
            Signal::WINCH,
            Signal::SYS,
            unsafe { Signal::from_raw_unchecked(libc::SIGRTMIN()) },
            unsafe { Signal::from_raw_unchecked(libc::SIGRTMIN() + 7) },
            unsafe { Signal::from_raw_unchecked(libc::SIGRTMAX()) },
        ];

        #[cfg(linux_raw)]
        {
            sigs.push(unsafe { Signal::from_raw_unchecked(KERNEL_SIGRTMIN) });
            sigs.push(unsafe { Signal::from_raw_unchecked(KERNEL_SIGRTMIN + 7) });
            sigs.push(unsafe { Signal::from_raw_unchecked(KERNEL_SIGRTMAX) });
        }

        sigs
    }

    /// A bunch of non-reserved signals for testing.
    fn libc_sigs() -> [Signal; 31] {
        [
            Signal::HUP,
            Signal::INT,
            Signal::QUIT,
            Signal::ILL,
            Signal::TRAP,
            Signal::ABORT,
            Signal::BUS,
            Signal::FPE,
            Signal::KILL,
            Signal::USR1,
            Signal::SEGV,
            Signal::USR2,
            Signal::PIPE,
            Signal::ALARM,
            Signal::TERM,
            Signal::CHILD,
            Signal::CONT,
            Signal::STOP,
            Signal::TSTP,
            Signal::TTIN,
            Signal::TTOU,
            Signal::URG,
            Signal::XCPU,
            Signal::XFSZ,
            Signal::VTALARM,
            Signal::PROF,
            Signal::WINCH,
            Signal::SYS,
            unsafe { Signal::from_raw_unchecked(libc::SIGRTMIN()) },
            unsafe { Signal::from_raw_unchecked(libc::SIGRTMIN() + 7) },
            unsafe { Signal::from_raw_unchecked(libc::SIGRTMAX()) },
        ]
    }

    #[test]
    fn test_ops_plain() {
        for sig in sigs() {
            let mut set = KernelSigSet::empty();
            for sig in sigs() {
                assert!(!set.contains(sig));
            }

            set.insert(sig);
            assert!(set.contains(sig));
            for sig in sigs().iter().filter(|s| **s != sig) {
                assert!(!set.contains(*sig));
            }

            set.remove(sig);
            for sig in sigs() {
                assert!(!set.contains(sig));
            }
        }
    }

    #[test]
    fn test_clear() {
        let mut set = KernelSigSet::empty();
        for sig in sigs() {
            set.insert(sig);
        }

        set.clear();

        for sig in sigs() {
            assert!(!set.contains(sig));
        }
    }

    // io_uring libraries assume that libc's `sigset_t` matches the layout
    // of the Linux kernel's `kernel_sigset_t`. Test that rustix's layout
    // matches as well.
    #[test]
    fn test_libc_layout_compatibility() {
        use crate::utils::as_ptr;

        let mut lc = unsafe { core::mem::zeroed::<libc::sigset_t>() };
        let mut ru = KernelSigSet::empty();
        let r = unsafe { libc::sigemptyset(&mut lc) };

        assert_eq!(r, 0);
        assert_eq!(
            unsafe {
                libc::memcmp(
                    as_ptr(&lc).cast(),
                    as_ptr(&ru).cast(),
                    core::mem::size_of::<KernelSigSet>(),
                )
            },
            0
        );

        for sig in libc_sigs() {
            ru.insert(sig);
            assert_ne!(
                unsafe {
                    libc::memcmp(
                        as_ptr(&lc).cast(),
                        as_ptr(&ru).cast(),
                        core::mem::size_of::<KernelSigSet>(),
                    )
                },
                0
            );
            let r = unsafe { libc::sigaddset(&mut lc, sig.as_raw()) };
            assert_eq!(r, 0);
            assert_eq!(
                unsafe {
                    libc::memcmp(
                        as_ptr(&lc).cast(),
                        as_ptr(&ru).cast(),
                        core::mem::size_of::<KernelSigSet>(),
                    )
                },
                0
            );
            ru.remove(sig);
            assert_ne!(
                unsafe {
                    libc::memcmp(
                        as_ptr(&lc).cast(),
                        as_ptr(&ru).cast(),
                        core::mem::size_of::<KernelSigSet>(),
                    )
                },
                0
            );
            let r = unsafe { libc::sigdelset(&mut lc, sig.as_raw()) };
            assert_eq!(r, 0);
            assert_eq!(
                unsafe {
                    libc::memcmp(
                        as_ptr(&lc).cast(),
                        as_ptr(&ru).cast(),
                        core::mem::size_of::<KernelSigSet>(),
                    )
                },
                0
            );
        }
    }
}
