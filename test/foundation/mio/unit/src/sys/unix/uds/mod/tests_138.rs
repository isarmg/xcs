#[cfg(test)]
mod tests {
    use std::os::unix::net::SocketAddr;
    use std::path::Path;
    use std::str;

    use super::{path_offset, unix_addr};

    #[test]
    fn pathname_address() {
        const PATH: &str = "./foo/bar.txt";
        const PATH_LEN: usize = 13;

        // Pathname addresses do have a null terminator, so `socklen` is
        // expected to be `PATH_LEN` + `offset` + 1.
        let address = SocketAddr::from_pathname(Path::new(PATH)).unwrap();
        let (sockaddr, actual) = unix_addr(&address);
        let offset = path_offset(&sockaddr);
        let expected = PATH_LEN + offset + 1;
        assert_eq!(expected as libc::socklen_t, actual)
    }

    #[test]
    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn abstract_address() {
        #[cfg(target_os = "android")]
        use std::os::android::net::SocketAddrExt;
        #[cfg(target_os = "linux")]
        use std::os::linux::net::SocketAddrExt;

        const PATH: &[u8] = &[0, 116, 111, 107, 105, 111];
        const PATH_LEN: usize = 6;

        // Abstract addresses do not have a null terminator, so `socklen` is
        // expected to be `PATH_LEN` + `offset`.
        let address = SocketAddr::from_abstract_name(PATH).unwrap();
        let (sockaddr, actual) = unix_addr(&address);
        let offset = path_offset(&sockaddr);
        let expected = PATH_LEN + offset;
        assert_eq!(expected as libc::socklen_t, actual)
    }
}
