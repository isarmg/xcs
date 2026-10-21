#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basics() {
        assert_eq!(Signal::HUP.as_raw(), libc::SIGHUP);
        unsafe {
            assert_eq!(Signal::from_raw_unchecked(libc::SIGHUP), Signal::HUP);
            assert_eq!(
                Signal::from_raw_nonzero_unchecked(NonZeroI32::new(libc::SIGHUP).unwrap()),
                Signal::HUP
            );
        }
    }

    #[test]
    fn test_named() {
        assert_eq!(Signal::from_named_raw(-1), None);
        assert_eq!(Signal::from_named_raw(0), None);
        assert_eq!(Signal::from_named_raw(c::SIGHUP), Some(Signal::HUP));
        assert_eq!(Signal::from_named_raw(c::SIGSEGV), Some(Signal::SEGV));
        assert_eq!(Signal::from_named_raw(c::SIGSYS), Some(Signal::SYS));
        #[cfg(any(linux_like, solarish, target_os = "hurd"))]
        {
            assert_eq!(Signal::from_named_raw(libc::SIGRTMIN()), None);
            assert_eq!(Signal::from_named_raw(libc::SIGRTMAX()), None);
        }
    }
}
