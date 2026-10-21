#[cfg(test)]
mod test {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn negotiation_req_serialization() {
        let expected = [
            0x05, // protocol version
            0x01, // number of authentication methods: 1
            0x00, // method: no authentication
        ];

        let mut buf = BytesMut::with_capacity(expected.len());
        let n = NegotiationReq(&AuthMethod::NoAuth)
            .write_to_buf(&mut buf)
            .unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn negotiation_res_deserialization() {
        let raw = [
            0x05, // protocol version
            0x02, // selected method: username/password
        ];

        let mut view = &raw[..];

        let res = NegotiationRes::try_from(&mut view).unwrap();
        assert_eq!(res, NegotiationRes(AuthMethod::UserPass));
        assert!(view.is_empty());
    }

    #[test]
    fn authentication_req_serialization() {
        let expected = [
            0x01, // authentication version
            0x04, // username length: 4
            b'u', b's', b'e', b'r', // username
            0x04, // password length: 4
            b'p', b'a', b's', b's', // password
        ];

        let mut buf = BytesMut::with_capacity(expected.len());
        let n = AuthenticationReq("user", "pass")
            .write_to_buf(&mut buf)
            .unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn authentication_res_deserialization() {
        let raw = [
            0x01, // authentication version
            0x00, // status: success
        ];
        assert_eq!(
            AuthenticationRes::try_from(&mut &raw[..]).unwrap(),
            AuthenticationRes(true)
        );

        let raw = [
            0x01, // authentication version
            0x01, // status: failure
        ];
        assert_eq!(
            AuthenticationRes::try_from(&mut &raw[..]).unwrap(),
            AuthenticationRes(false)
        );
    }

    #[test]
    fn proxy_req_serialization() {
        let expected = [
            0x05, // protocol version
            0x01, // command: connect
            0x00, // reserved
            0x01, // address type: IPv4
            127, 0, 0, 1, // destination address: 127.0.0.1
            0x1F, 0x90, // destination port: 8080
        ];

        let addr = Address::Socket(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080));

        let mut buf = BytesMut::with_capacity(expected.len());
        let n = ProxyReq(&addr).write_to_buf(&mut buf).unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn proxy_res_deserialization() {
        let raw = [
            0x05, // protocol version
            0x00, // reply: success
            0x00, // reserved
            0x01, // address type: IPv4
            127, 0, 0, 1, // bound address: 127.0.0.1
            0x1F, 0x90, // bound port: 8080
        ];
        let mut view = &raw[..];

        let res = ProxyRes::try_from(&mut view).unwrap();
        assert_eq!(res, ProxyRes(Status::Success));
        assert!(view.is_empty());
    }

    #[test]
    fn serialization_would_overflow() {
        let mut buf = BytesMut::with_capacity(2);

        let err = NegotiationReq(&AuthMethod::NoAuth)
            .write_to_buf(&mut buf)
            .unwrap_err();
        assert!(matches!(err, SerializeError::WouldOverflow));
    }

    fn assert_address_roundtrips(addr: Address) {
        let mut buf = BytesMut::with_capacity(64);

        let n = addr.write_to_buf(&mut buf).unwrap();
        assert_eq!(n, buf.len());

        let mut view = &buf[..];
        assert_eq!(Address::try_from(&mut view).unwrap(), addr);
        assert!(view.is_empty(), "address bytes should be fully consumed");
    }

    #[test]
    fn address_roundtrip_ipv4() {
        assert_address_roundtrips(Address::Socket(SocketAddr::new(
            Ipv4Addr::LOCALHOST.into(),
            8080,
        )));
    }

    #[test]
    fn address_roundtrip_ipv6() {
        assert_address_roundtrips(Address::Socket(SocketAddr::new(
            Ipv6Addr::LOCALHOST.into(),
            8080,
        )));
    }

    #[test]
    fn address_roundtrip_domain() {
        assert_address_roundtrips(Address::Domain("example.com".into(), 8080));
    }
}
