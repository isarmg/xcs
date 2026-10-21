#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_funcs() {
        // `TUNGETDEVNETNS` is defined as `_IO('T', 227)`.
        assert_eq!(
            linux_raw_sys::ioctl::TUNGETDEVNETNS as Opcode,
            opcode::none(b'T', 227)
        );
        // `FS_IOC_GETVERSION` is defined as `_IOR('v', 1, long)`.
        assert_eq!(
            linux_raw_sys::ioctl::FS_IOC_GETVERSION as Opcode,
            opcode::read::<c::c_long>(b'v', 1)
        );
        // `TUNSETNOCSUM` is defined as `_IOW('T', 200, int)`.
        assert_eq!(
            linux_raw_sys::ioctl::TUNSETNOCSUM as Opcode,
            opcode::write::<c::c_int>(b'T', 200)
        );
        // `FIFREEZE` is defined as `_IOWR('X', 119, int)`.
        assert_eq!(
            linux_raw_sys::ioctl::FIFREEZE as Opcode,
            opcode::read_write::<c::c_int>(b'X', 119)
        );
    }
}
