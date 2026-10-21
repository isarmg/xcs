#[cfg(test)]
mod tests {
    use self::StandardHeader::Vary;
    use super::*;

    #[test]
    fn test_bounds() {
        fn check_bounds<T: Sync + Send>() {}
        check_bounds::<HeaderName>();
    }

    #[test]
    fn test_parse_invalid_headers() {
        for i in 0..128 {
            let hdr = vec![1u8; i];
            assert!(
                HeaderName::from_bytes(&hdr).is_err(),
                "{} invalid header chars did not fail",
                i
            );
        }
    }

    const ONE_TOO_LONG: &[u8] = &[b'a'; super::super::MAX_HEADER_NAME_LEN + 1];

    #[test]
    fn test_invalid_name_lengths() {
        assert!(
            HeaderName::from_bytes(&[]).is_err(),
            "zero-length header name is an error",
        );

        let long = &ONE_TOO_LONG[0..super::super::MAX_HEADER_NAME_LEN];

        let long_str = std::str::from_utf8(long).unwrap();
        assert_eq!(HeaderName::from_static(long_str), long_str); // shouldn't panic!

        assert!(
            HeaderName::from_bytes(long).is_ok(),
            "max header name length is ok",
        );
        assert!(
            HeaderName::from_bytes(ONE_TOO_LONG).is_err(),
            "longer than max header name length is an error",
        );
    }

    #[test]
    #[should_panic]
    fn test_static_invalid_name_lengths() {
        // Safety: ONE_TOO_LONG contains only the UTF-8 safe, single-byte codepoint b'a'.
        let _ = HeaderName::from_static(unsafe { std::str::from_utf8_unchecked(ONE_TOO_LONG) });
    }

    #[test]
    fn test_from_hdr_name() {
        use self::StandardHeader::Vary;

        let name = HeaderName::from(HdrName {
            inner: Repr::Standard(Vary),
        });

        assert_eq!(name.inner, Repr::Standard(Vary));

        let name = HeaderName::from(HdrName {
            inner: Repr::Custom(MaybeLower {
                buf: b"hello-world",
                lower: true,
            }),
        });

        assert_eq!(
            name.inner,
            Repr::Custom(Custom(ByteStr::from_static("hello-world")))
        );

        let name = HeaderName::from(HdrName {
            inner: Repr::Custom(MaybeLower {
                buf: b"Hello-World",
                lower: false,
            }),
        });

        assert_eq!(
            name.inner,
            Repr::Custom(Custom(ByteStr::from_static("hello-world")))
        );
    }

    #[test]
    fn test_eq_hdr_name() {
        use self::StandardHeader::Vary;

        let a = HeaderName {
            inner: Repr::Standard(Vary),
        };
        let b = HdrName {
            inner: Repr::Standard(Vary),
        };

        assert_eq!(a, b);

        let a = HeaderName {
            inner: Repr::Custom(Custom(ByteStr::from_static("vaary"))),
        };
        assert_ne!(a, b);

        let b = HdrName {
            inner: Repr::Custom(MaybeLower {
                buf: b"vaary",
                lower: true,
            }),
        };

        assert_eq!(a, b);

        let b = HdrName {
            inner: Repr::Custom(MaybeLower {
                buf: b"vaary",
                lower: false,
            }),
        };

        assert_eq!(a, b);

        let b = HdrName {
            inner: Repr::Custom(MaybeLower {
                buf: b"VAARY",
                lower: false,
            }),
        };

        assert_eq!(a, b);

        let a = HeaderName {
            inner: Repr::Standard(Vary),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_from_static_std() {
        let a = HeaderName {
            inner: Repr::Standard(Vary),
        };

        let b = HeaderName::from_static("vary");
        assert_eq!(a, b);

        let b = HeaderName::from_static("vaary");
        assert_ne!(a, b);
    }

    #[test]
    #[should_panic]
    fn test_from_static_std_uppercase() {
        HeaderName::from_static("Vary");
    }

    #[test]
    #[should_panic]
    fn test_from_static_std_symbol() {
        HeaderName::from_static("vary{}");
    }

    // MaybeLower { lower: true }
    #[test]
    fn test_from_static_custom_short() {
        let a = HeaderName {
            inner: Repr::Custom(Custom(ByteStr::from_static("customheader"))),
        };
        let b = HeaderName::from_static("customheader");
        assert_eq!(a, b);
    }

    #[test]
    #[should_panic]
    fn test_from_static_custom_short_uppercase() {
        HeaderName::from_static("custom header");
    }

    #[test]
    #[should_panic]
    fn test_from_static_custom_short_symbol() {
        HeaderName::from_static("CustomHeader");
    }

    // MaybeLower { lower: false }
    #[test]
    fn test_from_static_custom_long() {
        let a = HeaderName {
            inner: Repr::Custom(Custom(ByteStr::from_static(
                "longer-than-63--thisheaderislongerthansixtythreecharactersandthushandleddifferent",
            ))),
        };
        let b = HeaderName::from_static(
            "longer-than-63--thisheaderislongerthansixtythreecharactersandthushandleddifferent",
        );
        assert_eq!(a, b);
    }

    #[test]
    #[should_panic]
    fn test_from_static_custom_long_uppercase() {
        HeaderName::from_static(
            "Longer-Than-63--ThisHeaderIsLongerThanSixtyThreeCharactersAndThusHandledDifferent",
        );
    }

    #[test]
    #[should_panic]
    fn test_from_static_custom_long_symbol() {
        HeaderName::from_static(
            "longer-than-63--thisheader{}{}{}{}islongerthansixtythreecharactersandthushandleddifferent"
        );
    }

    #[test]
    fn test_from_static_custom_single_char() {
        let a = HeaderName {
            inner: Repr::Custom(Custom(ByteStr::from_static("a"))),
        };
        let b = HeaderName::from_static("a");
        assert_eq!(a, b);
    }

    #[test]
    #[should_panic]
    fn test_from_static_empty() {
        HeaderName::from_static("");
    }

    #[test]
    fn test_all_tokens() {
        HeaderName::from_static("!#$%&'*+-.^_`|~0123456789abcdefghijklmnopqrstuvwxyz");
    }

    #[test]
    fn test_from_lowercase() {
        HeaderName::from_lowercase(&[0; 10]).unwrap_err();
        HeaderName::from_lowercase(&[b'A'; 10]).unwrap_err();
        HeaderName::from_lowercase(&[0x1; 10]).unwrap_err();
        HeaderName::from_lowercase(&[0xFF; 10]).unwrap_err();
        //HeaderName::from_lowercase(&[0; 100]).unwrap_err();
        HeaderName::from_lowercase(&[b'A'; 100]).unwrap_err();
        HeaderName::from_lowercase(&[0x1; 100]).unwrap_err();
        HeaderName::from_lowercase(&[0xFF; 100]).unwrap_err();
    }
}
