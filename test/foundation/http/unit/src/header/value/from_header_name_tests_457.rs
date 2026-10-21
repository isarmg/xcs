#[cfg(test)]
mod from_header_name_tests {
    use super::*;
    use crate::header::map::HeaderMap;
    use crate::header::name;

    #[test]
    fn it_can_insert_header_name_as_header_value() {
        let mut map = HeaderMap::new();
        map.insert(name::UPGRADE, name::SEC_WEBSOCKET_PROTOCOL.into());
        map.insert(
            name::ACCEPT,
            name::HeaderName::from_bytes(b"hello-world").unwrap().into(),
        );

        assert_eq!(
            map.get(name::UPGRADE).unwrap(),
            HeaderValue::from_bytes(b"sec-websocket-protocol").unwrap()
        );

        assert_eq!(
            map.get(name::ACCEPT).unwrap(),
            HeaderValue::from_bytes(b"hello-world").unwrap()
        );
    }
}
