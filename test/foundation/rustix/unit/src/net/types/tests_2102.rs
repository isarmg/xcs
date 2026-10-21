#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes() {
        #[cfg(target_os = "linux")]
        use crate::backend::c;
        use crate::ffi::c_int;
        use crate::net::addr::SocketAddrStorage;
        use core::mem::transmute;

        // Backend code needs to cast these to `c_int` so make sure that cast isn't
        // lossy.
        static_assertions::assert_eq_size!(RawProtocol, c_int);
        static_assertions::assert_eq_size!(Protocol, c_int);
        static_assertions::assert_eq_size!(Option<RawProtocol>, c_int);
        static_assertions::assert_eq_size!(Option<Protocol>, c_int);
        static_assertions::assert_eq_size!(RawSocketType, c_int);
        static_assertions::assert_eq_size!(SocketType, c_int);
        static_assertions::assert_eq_size!(SocketFlags, c_int);
        static_assertions::assert_eq_size!(SocketAddrStorage, c::sockaddr_storage);

        // Rustix doesn't depend on `Option<Protocol>` matching the ABI of a raw
        // integer for correctness, but it should work nonetheless.
        #[allow(unsafe_code)]
        unsafe {
            let t: Option<Protocol> = None;
            assert_eq!(0_u32, transmute::<Option<Protocol>, u32>(t));

            let t: Option<Protocol> = Some(Protocol::from_raw(RawProtocol::new(4567).unwrap()));
            assert_eq!(4567_u32, transmute::<Option<Protocol>, u32>(t));
        }

        #[cfg(linux_kernel)]
        static_assertions::assert_eq_size!(UCred, libc::ucred);

        #[cfg(target_os = "linux")]
        static_assertions::assert_eq_size!(super::xdp::XdpUmemReg, c::xdp_umem_reg);
        #[cfg(target_os = "linux")]
        static_assertions::assert_eq_size!(super::xdp::XdpOptions, c::xdp_options);
        #[cfg(target_os = "linux")]
        static_assertions::assert_eq_size!(super::xdp::XdpDesc, c::xdp_desc);
    }
}
