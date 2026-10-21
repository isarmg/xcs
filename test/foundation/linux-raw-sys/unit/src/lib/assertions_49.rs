#[cfg(test)]
mod assertions {
    use super::ctypes;

    macro_rules! same_c_type {
        ($($ty:ident),+ $(,)?) => { $(
            const _: fn(ctypes::$ty) -> core::ffi::$ty = |value| value;
            const _: fn(core::ffi::$ty) -> ctypes::$ty = |value| value;
        )+ };
    }

    same_c_type!(
        c_char,
        c_schar,
        c_uchar,
        c_short,
        c_ushort,
        c_int,
        c_uint,
        c_long,
        c_ulong,
        c_longlong,
        c_ulonglong,
        c_float,
        c_double,
    );

    #[test]
    fn retained_c_types_match_linux_amd64_abi() {
        assert_eq!(core::mem::size_of::<ctypes::c_char>(), 1);
        assert_eq!(ctypes::c_char::MIN, -128);
        assert_eq!(core::mem::size_of::<ctypes::c_long>(), 8);
        assert_eq!(core::mem::align_of::<ctypes::c_long>(), 8);
        assert_eq!(core::mem::size_of::<ctypes::c_ulong>(), 8);
    }

    #[cfg(feature = "general")]
    #[test]
    fn retained_fd_set_helpers_preserve_bit_boundaries() {
        use super::{general::__kernel_fd_set, select_macros::*};
        // The Linux ABI represents fd_set as an integer array; all-zero is valid.
        let mut set: __kernel_fd_set = unsafe { core::mem::zeroed() };
        let last = (core::mem::size_of::<__kernel_fd_set>() * 8 - 1) as ctypes::c_int;
        unsafe {
            for fd in [0, 7, 8, 63, 64, last] {
                FD_SET(fd, &mut set);
                assert!(FD_ISSET(fd, &set));
                FD_CLR(fd, &mut set);
                assert!(!FD_ISSET(fd, &set));
            }
            FD_SET(-1, &mut set);
            assert!(!FD_ISSET(-1, &set));
            FD_SET(last, &mut set);
            FD_ZERO(&mut set);
            assert!(!FD_ISSET(last, &set));
        }
    }

    #[cfg(feature = "net")]
    #[test]
    fn retained_control_message_helpers_preserve_alignment() {
        use super::{cmsg_macros::*, net::cmsghdr};
        let header = core::mem::size_of::<cmsghdr>() as ctypes::c_uint;
        unsafe {
            assert_eq!(CMSG_ALIGN(0), 0);
            assert_eq!(CMSG_ALIGN(1), 8);
            assert_eq!(CMSG_ALIGN(8), 8);
            assert_eq!(CMSG_ALIGN(9), 16);
            assert_eq!(CMSG_LEN(1), header + 1);
            assert_eq!(CMSG_SPACE(1), header + 8);
        }
    }
}
