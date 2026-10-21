#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_method_eq() {
        assert_eq!(Method::GET, Method::GET);
        assert_eq!(Method::GET, "GET");
        assert_eq!(&Method::GET, "GET");

        assert_eq!("GET", Method::GET);
        assert_eq!("GET", &Method::GET);

        assert_eq!(&Method::GET, Method::GET);
        assert_eq!(Method::GET, &Method::GET);
    }

    #[test]
    fn test_invalid_method() {
        assert!(Method::from_str("").is_err());
        assert!(Method::from_bytes(b"").is_err());
        assert!(Method::from_bytes(&[0xC0]).is_err()); // invalid utf-8
        assert!(Method::from_bytes(&[0x10]).is_err()); // invalid method characters
    }

    #[test]
    fn test_is_idempotent() {
        assert!(Method::OPTIONS.is_idempotent());
        assert!(Method::GET.is_idempotent());
        assert!(Method::PUT.is_idempotent());
        assert!(Method::DELETE.is_idempotent());
        assert!(Method::HEAD.is_idempotent());
        assert!(Method::TRACE.is_idempotent());
        assert!(Method::QUERY.is_idempotent());

        assert!(!Method::POST.is_idempotent());
        assert!(!Method::CONNECT.is_idempotent());
        assert!(!Method::PATCH.is_idempotent());
    }

    #[test]
    fn test_extension_method() {
        assert_eq!(Method::from_str("WOW").unwrap(), "WOW");
        assert_eq!(Method::from_str("wOw!!").unwrap(), "wOw!!");

        let long_method = "This_is_a_very_long_method.It_is_valid_but_unlikely.";
        assert_eq!(Method::from_str(long_method).unwrap(), long_method);

        let longest_inline_method = [b'A'; InlineExtension::MAX];
        assert_eq!(
            Method::from_bytes(&longest_inline_method).unwrap(),
            Method(ExtensionInline(
                InlineExtension::new(&longest_inline_method).unwrap()
            ))
        );
        let shortest_allocated_method = [b'A'; InlineExtension::MAX + 1];
        assert_eq!(
            Method::from_bytes(&shortest_allocated_method).unwrap(),
            Method(ExtensionAllocated(
                AllocatedExtension::new(&shortest_allocated_method).unwrap()
            ))
        );
    }

    #[test]
    fn test_extension_method_chars() {
        const VALID_METHOD_CHARS: &str =
            "!#$%&'*+-.^_`|~0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

        for c in VALID_METHOD_CHARS.chars() {
            let c = c.to_string();

            assert_eq!(
                Method::from_str(&c).unwrap(),
                c.as_str(),
                "testing {c} is a valid method character"
            );
        }
    }
}
