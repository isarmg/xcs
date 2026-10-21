#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // For new `ToValue` implementations, also add a test to the `tests/macros` file

    impl<'v> Value<'v> {
        #[allow(mismatched_lifetime_syntaxes)]
        pub(crate) fn to_token(&self) -> inner::Token {
            self.inner.to_test_token()
        }
    }

    fn unsigned() -> impl Iterator<Item = Value<'static>> {
        vec![
            Value::from(8u8),
            Value::from(16u16),
            Value::from(32u32),
            Value::from(64u64),
            Value::from(1usize),
            Value::from(std::num::NonZeroU8::new(8).unwrap()),
            Value::from(std::num::NonZeroU16::new(16).unwrap()),
            Value::from(std::num::NonZeroU32::new(32).unwrap()),
            Value::from(std::num::NonZeroU64::new(64).unwrap()),
            Value::from(std::num::NonZeroUsize::new(1).unwrap()),
        ]
        .into_iter()
    }

    fn signed() -> impl Iterator<Item = Value<'static>> {
        vec![
            Value::from(-8i8),
            Value::from(-16i16),
            Value::from(-32i32),
            Value::from(-64i64),
            Value::from(-1isize),
            Value::from(std::num::NonZeroI8::new(-8).unwrap()),
            Value::from(std::num::NonZeroI16::new(-16).unwrap()),
            Value::from(std::num::NonZeroI32::new(-32).unwrap()),
            Value::from(std::num::NonZeroI64::new(-64).unwrap()),
            Value::from(std::num::NonZeroIsize::new(-1).unwrap()),
        ]
        .into_iter()
    }

    fn float() -> impl Iterator<Item = Value<'static>> {
        vec![Value::from(32.32f32), Value::from(64.64f64)].into_iter()
    }

    fn bool() -> impl Iterator<Item = Value<'static>> {
        vec![Value::from(true), Value::from(false)].into_iter()
    }

    fn str() -> impl Iterator<Item = Value<'static>> {
        vec![Value::from("a string"), Value::from("a loong string")].into_iter()
    }

    fn char() -> impl Iterator<Item = Value<'static>> {
        vec![Value::from('a'), Value::from('⛰')].into_iter()
    }

    #[test]
    fn test_to_value_display() {
        assert_eq!(42u64.to_value().to_string(), "42");
        assert_eq!(42i64.to_value().to_string(), "42");
        assert_eq!(42.01f64.to_value().to_string(), "42.01");
        assert_eq!(true.to_value().to_string(), "true");
        assert_eq!('a'.to_value().to_string(), "a");
        assert_eq!("a loong string".to_value().to_string(), "a loong string");
        assert_eq!(Some(true).to_value().to_string(), "true");
        assert_eq!(().to_value().to_string(), "None");
        assert_eq!(None::<bool>.to_value().to_string(), "None");
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_net_to_value_display() {
        use std::str::FromStr;

        assert_eq!(
            std::net::Ipv4Addr::new(192, 168, 10, 100)
                .to_value()
                .to_string(),
            "192.168.10.100"
        );
        assert_eq!(
            std::net::Ipv6Addr::from_str("f33c::1")
                .unwrap()
                .to_value()
                .to_string(),
            "f33c::1"
        );
        assert_eq!(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 10, 100))
                .to_value()
                .to_string(),
            "192.168.10.100"
        );
        assert_eq!(
            std::net::IpAddr::V6(std::net::Ipv6Addr::from_str("f33c::1").unwrap())
                .to_value()
                .to_string(),
            "f33c::1"
        );
        assert_eq!(
            std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(192, 168, 10, 100), 12345)
                .to_value()
                .to_string(),
            "192.168.10.100:12345"
        );
        assert_eq!(
            std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::from_str("f33c::1").unwrap(),
                12345,
                0,
                0
            )
            .to_value()
            .to_string(),
            "[f33c::1]:12345"
        );
        assert_eq!(
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(192, 168, 10, 100),
                12345
            ))
            .to_value()
            .to_string(),
            "192.168.10.100:12345"
        );
        assert_eq!(
            std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::from_str("f33c::1").unwrap(),
                12345,
                0,
                0
            ))
            .to_value()
            .to_string(),
            "[f33c::1]:12345"
        );
    }

    #[test]
    fn test_to_value_structured() {
        assert_eq!(42u64.to_value().to_token(), inner::Token::U64(42));
        assert_eq!(42i64.to_value().to_token(), inner::Token::I64(42));
        assert_eq!(42.01f64.to_value().to_token(), inner::Token::F64(42.01));
        assert_eq!(true.to_value().to_token(), inner::Token::Bool(true));
        assert_eq!('a'.to_value().to_token(), inner::Token::Char('a'));
        assert_eq!(
            "a loong string".to_value().to_token(),
            inner::Token::Str("a loong string".into())
        );
        assert_eq!(Some(true).to_value().to_token(), inner::Token::Bool(true));
        assert_eq!(().to_value().to_token(), inner::Token::None);
        assert_eq!(None::<bool>.to_value().to_token(), inner::Token::None);
    }

    #[test]
    fn test_to_number() {
        for v in unsigned() {
            assert!(v.to_u64().is_some());
            assert!(v.to_i64().is_some());
        }

        for v in signed() {
            assert!(v.to_i64().is_some());
        }

        for v in unsigned().chain(signed()).chain(float()) {
            assert!(v.to_f64().is_some());
        }

        for v in bool().chain(str()).chain(char()) {
            assert!(v.to_u64().is_none());
            assert!(v.to_i64().is_none());
            assert!(v.to_f64().is_none());
        }
    }

    #[test]
    fn test_to_float() {
        // Only integers from i32::MIN..=u32::MAX can be converted into floats
        assert!(Value::from(i32::MIN).to_f64().is_some());
        assert!(Value::from(u32::MAX).to_f64().is_some());

        assert!(Value::from((i32::MIN as i64) - 1).to_f64().is_none());
        assert!(Value::from((u32::MAX as u64) + 1).to_f64().is_none());
    }

    #[test]
    fn test_to_cow_str() {
        for v in str() {
            assert!(v.to_borrowed_str().is_some());

            #[cfg(feature = "kv_std")]
            assert!(v.to_cow_str().is_some());
        }

        let short_lived = String::from("short lived");
        let v = Value::from(&*short_lived);

        assert!(v.to_borrowed_str().is_some());

        #[cfg(feature = "kv_std")]
        assert!(v.to_cow_str().is_some());

        for v in unsigned().chain(signed()).chain(float()).chain(bool()) {
            assert!(v.to_borrowed_str().is_none());

            #[cfg(feature = "kv_std")]
            assert!(v.to_cow_str().is_none());
        }
    }

    #[test]
    fn test_to_bool() {
        for v in bool() {
            assert!(v.to_bool().is_some());
        }

        for v in unsigned()
            .chain(signed())
            .chain(float())
            .chain(str())
            .chain(char())
        {
            assert!(v.to_bool().is_none());
        }
    }

    #[test]
    fn test_to_char() {
        for v in char() {
            assert!(v.to_char().is_some());
        }

        for v in unsigned()
            .chain(signed())
            .chain(float())
            .chain(str())
            .chain(bool())
        {
            assert!(v.to_char().is_none());
        }
    }

    #[test]
    fn test_visit_integer() {
        struct Extract(Option<u64>);

        impl<'v> VisitValue<'v> for Extract {
            fn visit_any(&mut self, value: Value) -> Result<(), Error> {
                unimplemented!("unexpected value: {value:?}")
            }

            fn visit_u64(&mut self, value: u64) -> Result<(), Error> {
                self.0 = Some(value);

                Ok(())
            }
        }

        let mut extract = Extract(None);
        Value::from(42u64).visit(&mut extract).unwrap();

        assert_eq!(Some(42), extract.0);
    }

    #[test]
    fn test_visit_borrowed_str() {
        struct Extract<'v>(Option<&'v str>);

        impl<'v> VisitValue<'v> for Extract<'v> {
            fn visit_any(&mut self, value: Value) -> Result<(), Error> {
                unimplemented!("unexpected value: {value:?}")
            }

            fn visit_borrowed_str(&mut self, value: &'v str) -> Result<(), Error> {
                self.0 = Some(value);

                Ok(())
            }
        }

        let mut extract = Extract(None);

        let short_lived = String::from("A short-lived string");
        Value::from(&*short_lived).visit(&mut extract).unwrap();

        assert_eq!(Some("A short-lived string"), extract.0);
    }
}
