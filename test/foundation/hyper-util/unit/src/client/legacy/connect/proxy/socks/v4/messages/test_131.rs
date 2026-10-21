#[cfg(test)]
mod test {
    use super::*;
    use bytes::BytesMut;
    use std::net::Ipv4Addr;

    #[test]
    fn request_serialization_with_socket() {
        let expected = [
            0x04, // protocol version
            0x01, // command: connect
            0x1F, 0x90, // destination port: 8080
            127, 0, 0, 1,    // destination address: 127.0.0.1
            0x00, // null terminating an empty userid
        ];

        let addr = Address::Socket(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080));
        let mut buf = BytesMut::with_capacity(expected.len());
        let n = Request(&addr).write_to_buf(&mut buf).unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn request_serialization_with_domain() {
        let expected = [
            0x04, // protocol version
            0x01, // command: connect
            0x1F, 0x90, // destination port: 8080
            0x00, 0x00, 0x00, 0xFF, // invalid IP: signals that a domain follows (SOCKS4a)
            0x00, // null terminating an empty userid
            b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm', // domain
            0x00, // null terminator
        ];

        let addr = Address::Domain("example.com".into(), 8080);
        let mut buf = BytesMut::with_capacity(expected.len());
        let n = Request(&addr).write_to_buf(&mut buf).unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn response_deserialization() {
        let raw = [
            0x00, // reply version
            90,   // status: request granted
            0x1F, 0x90, // port: 8080 (ignored, only used for BIND)
            127, 0, 0, 1, // address: 127.0.0.1 (ignored, only used for BIND)
        ];
        let mut view = &raw[..];

        let res = Response::try_from(&mut view).unwrap();
        assert_eq!(res, Response(Status::Success));
        assert!(view.is_empty());
    }

    #[test]
    fn response_incomplete() {
        let raw = [
            0x00, // reply version
            90,   // status: request granted
            0x00, // truncated mid-port
        ];

        let err = Response::try_from(&mut &raw[..]).unwrap_err();
        assert!(matches!(err, ParsingError::Incomplete));
    }

    #[test]
    fn request_serialization_would_overflow() {
        let addr = Address::Socket(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080));
        let mut short = [0u8; 5];
        let err = Request(&addr).write_to_buf(&mut short[..]).unwrap_err();
        assert!(matches!(err, SerializeError::WouldOverflow));
    }
}
