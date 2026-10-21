#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(apple)]
    #[test]
    fn test_negative_timestamps() {
        let mut secs = -59;
        let mut nsecs = -900_000_000;
        (secs, nsecs) = fix_negative_nsecs(secs, nsecs);
        assert_eq!(secs, -60);
        assert_eq!(nsecs, 100_000_000);
        (secs, nsecs) = fix_negative_nsecs(secs, nsecs);
        assert_eq!(secs, -60);
        assert_eq!(nsecs, 100_000_000);
    }

    #[test]
    fn test_sizes() {
        static_assertions::assert_eq_size!(Secs, u64);
        static_assertions::const_assert!(
            core::mem::size_of::<Timespec>() >= core::mem::size_of::<(u64, u32)>()
        );
        static_assertions::const_assert!(core::mem::size_of::<Nsecs>() >= 4);

        let mut t = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        // `tv_nsec` needs to be able to hold nanoseconds up to a second.
        t.tv_nsec = 999_999_999_u32 as _;
        assert_eq!(t.tv_nsec as u64, 999_999_999_u64);

        // `tv_sec` needs to be able to hold more than 32-bits of seconds.
        t.tv_sec = 0x1_0000_0000_u64 as _;
        assert_eq!(t.tv_sec as u64, 0x1_0000_0000_u64);
    }

    #[cfg(any(libc, target_os = "wasi", windows))]
    #[test]
    fn test_to_timeval() {
        let ts = Timespec {
            tv_sec: 4,
            tv_nsec: 999_999_500,
        };
        let tv = ts.to_timeval().unwrap();
        assert_eq!(tv.tv_sec, 5);
        assert_eq!(tv.tv_usec, 0);

        let ts2 = Timespec {
            tv_sec: 2,
            tv_nsec: 500_000_000,
        };
        let tv2 = ts2.to_timeval().unwrap();
        assert_eq!(tv2.tv_sec, 2);
        assert_eq!(tv2.tv_usec, 500_000);
    }

    // Test that our workarounds are needed.
    #[cfg(fix_y2038)]
    #[test]
    #[allow(deprecated)]
    fn test_fix_y2038() {
        static_assertions::assert_eq_size!(libc::time_t, u32);
    }

    // Test that our workarounds are not needed.
    #[cfg(not(fix_y2038))]
    #[test]
    fn timespec_layouts() {
        use crate::backend::c;
        check_renamed_struct!(Timespec, timespec, tv_sec, tv_nsec);
    }

    // Test that `Timespec` matches Linux's `__kernel_timespec`.
    #[cfg(linux_raw_dep)]
    #[test]
    fn test_against_kernel_timespec() {
        static_assertions::assert_eq_size!(Timespec, linux_raw_sys::general::__kernel_timespec);
        static_assertions::assert_eq_align!(Timespec, linux_raw_sys::general::__kernel_timespec);
        assert_eq!(
            memoffset::span_of!(Timespec, tv_sec),
            memoffset::span_of!(linux_raw_sys::general::__kernel_timespec, tv_sec)
        );
        assert_eq!(
            memoffset::span_of!(Timespec, tv_nsec),
            memoffset::span_of!(linux_raw_sys::general::__kernel_timespec, tv_nsec)
        );
    }
}
