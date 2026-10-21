#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fmt::*,
        std::{str::FromStr, string::ToString},
        tests::some_uuid_iter,
    };

    #[test]
    fn test_parse_valid() {
        let from_hyphenated = Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
        let from_simple = Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c8").unwrap();
        let from_urn = Uuid::parse_str("urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
        let from_guid = Uuid::parse_str("{67e55044-10b1-426f-9247-bb680e5fe0c8}").unwrap();

        assert_eq!(from_hyphenated, from_simple);
        assert_eq!(from_hyphenated, from_urn);
        assert_eq!(from_hyphenated, from_guid);

        assert!(Uuid::parse_str("00000000000000000000000000000000").is_ok());
        assert!(Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E4").is_ok());
        assert!(Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("01020304-1112-2122-3132-414243444546").is_ok());
        assert!(Uuid::parse_str("urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("URN:UUID:67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("Urn:Uuid:67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("{6d93bade-bd9f-4e13-8914-9474e1e3567b}").is_ok());

        assert_eq!(
            from_urn,
            Uuid::parse_str("URN:UUID:67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap()
        );
        assert_eq!(
            from_urn,
            Uuid::parse_str("urn:UUID:67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap()
        );

        // Nil
        let nil = Uuid::nil();
        assert_eq!(
            Uuid::parse_str("00000000000000000000000000000000").unwrap(),
            nil
        );
        assert_eq!(
            Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
            nil
        );
    }

    #[test]
    fn test_parse_invalid() {
        // Invalid
        assert_eq!(
            Uuid::parse_str(""),
            Err(Error(ErrorKind::ParseLength { len: 0 }))
        );

        assert_eq!(
            Uuid::parse_str("{}"),
            Err(Error(ErrorKind::ParseGroupCount { count: 1 }))
        );

        assert_eq!(
            Uuid::parse_str("!"),
            Err(Error(ErrorKind::ParseChar {
                character: '!',
                index: 0,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E45"),
            Err(Error(ErrorKind::ParseGroupLength {
                group: 4,
                len: 13,
                index: 25,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-BBF-329BF39FA1E4"),
            Err(Error(ErrorKind::ParseGroupLength {
                group: 3,
                len: 3,
                index: 20,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-BGBF-329BF39FA1E4"),
            Err(Error(ErrorKind::ParseChar {
                character: 'G',
                index: 20,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2F4faaFB6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::ParseGroupCount { count: 2 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faaFB6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::ParseGroupCount { count: 3 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::ParseGroupCount { count: 4 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa"),
            Err(Error(ErrorKind::ParseGroupCount { count: 3 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faaXB6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::ParseChar {
                character: 'X',
                index: 18,
            }))
        );

        assert_eq!(
            Uuid::parse_str("{F9168C5E-CEB2-4faa9B6BFF329BF39FA1E41"),
            Err(Error(ErrorKind::ParseChar {
                character: '{',
                index: 0,
            }))
        );

        assert_eq!(
            Uuid::parse_str("{F9168C5E-CEB2-4faa9B6BFF329BF39FA1E41}"),
            Err(Error(ErrorKind::ParseGroupCount { count: 3 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB-24fa-eB6BFF32-BF39FA1E4"),
            Err(Error(ErrorKind::ParseGroupLength {
                group: 1,
                len: 3,
                index: 10,
            }))
        );

        // // (group, found, expecting)
        assert_eq!(
            Uuid::parse_str("01020304-1112-2122-3132-41424344"),
            Err(Error(ErrorKind::ParseGroupLength {
                group: 4,
                len: 8,
                index: 25,
            }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c"),
            Err(Error(ErrorKind::ParseLength { len: 31 }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c88"),
            Err(Error(ErrorKind::ParseLength { len: 33 }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0cg8"),
            Err(Error(ErrorKind::ParseChar {
                character: 'g',
                index: 31,
            }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426%9247bb680e5fe0c8"),
            Err(Error(ErrorKind::ParseChar {
                character: '%',
                index: 15,
            }))
        );

        assert_eq!(
            Uuid::parse_str("231231212212423424324323477343246663"),
            Err(Error(ErrorKind::ParseGroupCount { count: 1 }))
        );

        assert_eq!(
            Uuid::parse_str("{00000000000000000000000000000000}"),
            Err(Error(ErrorKind::ParseGroupCount { count: 1 }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c"),
            Err(Error(ErrorKind::ParseLength { len: 31 }))
        );

        assert_eq!(
            Uuid::parse_str("67e550X410b1426f9247bb680e5fe0cd"),
            Err(Error(ErrorKind::ParseChar {
                character: 'X',
                index: 6,
            }))
        );

        assert_eq!(
            Uuid::parse_str("67e550-4105b1426f9247bb680e5fe0c"),
            Err(Error(ErrorKind::ParseGroupCount { count: 2 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF1-02BF39FA1E4"),
            Err(Error(ErrorKind::ParseGroupLength {
                group: 3,
                len: 5,
                index: 20,
            }))
        );

        assert_eq!(
            Uuid::parse_str("\u{bcf3c}"),
            Err(Error(ErrorKind::ParseChar {
                character: '\u{bcf3c}',
                index: 0,
            }))
        );

        assert_eq!(
            Uuid::parse_str("\u{130}"),
            Err(Error(ErrorKind::ParseChar {
                character: '\u{130}',
                index: 0,
            }))
        );

        assert_eq!(
            Err(Error(ErrorKind::ParseLength { len: 0 })),
            Hyphenated::from_str("")
        );

        assert_eq!(
            Err(Error(ErrorKind::ParseGroupCount { count: 1 })),
            Hyphenated::from_str("550e8400e29b41d4a716446655440000")
        );

        assert_eq!(
            Err(Error(ErrorKind::ParseChar {
                character: '-',
                index: 8
            })),
            Simple::from_str("550e8400-e29b-41d4-a716-446655440000")
        );

        assert_eq!(
            Err(Error(ErrorKind::ParseChar {
                character: '5',
                index: 0
            })),
            Urn::from_str("550e8400-e29b-41d4-a716-446655440000")
        );
        assert_eq!(
            Err(Error(ErrorKind::ParseChar {
                character: ':',
                index: 0
            })),
            Urn::from_str(":550e8400-e29b-41d4-a716-446655440000")
        );
        assert_eq!(
            Uuid::parse_str("urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
            Urn::from_str("URN:UUID:67e55044-10b1-426f-9247-bb680e5fe0c8")
                .unwrap()
                .into_uuid()
        );

        assert_eq!(
            Err(Error(ErrorKind::ParseChar {
                character: '5',
                index: 0
            })),
            Braced::from_str("550e8400-e29b-41d4-a716-446655440000")
        );
        assert_eq!(
            Err(Error(ErrorKind::ParseChar {
                character: '{',
                index: 1
            })),
            Braced::from_str("{{550e8400-e29b-41d4-a716-446655440000}}")
        );

        // Unicode
        assert_eq!(
            Uuid::from_str("{6e0----------9=4O-0e5\u{14}e0c4\u{ec2f}8}"),
            Err(Error(ErrorKind::ParseChar {
                character: '=',
                index: 15,
            }))
        );

        assert_eq!(
            Uuid::from_str("urn:uuid:urae0c8"),
            Err(Error(ErrorKind::ParseChar {
                character: 'u',
                index: 9,
            }))
        );
    }

    #[test]
    fn test_roundtrip_default() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.to_string();
            let uuid_out = Uuid::parse_str(&orig_str).unwrap();
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_roundtrip_hyphenated() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.hyphenated().to_string();
            let uuid_out = Uuid::parse_str(&orig_str).unwrap();
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_roundtrip_simple() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.simple().to_string();
            let uuid_out = Uuid::parse_str(&orig_str).unwrap();
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_roundtrip_urn() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.urn().to_string();
            let uuid_out = Uuid::parse_str(&orig_str).unwrap();
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_roundtrip_braced() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.braced().to_string();
            let uuid_out = Uuid::parse_str(&orig_str).unwrap();
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_roundtrip_parse_urn() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.urn().to_string();
            let uuid_out = Uuid::from_bytes(parse_urn(orig_str.as_bytes()).unwrap());
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_roundtrip_parse_braced() {
        for uuid_orig in some_uuid_iter() {
            let orig_str = uuid_orig.braced().to_string();
            let uuid_out = Uuid::from_bytes(parse_braced(orig_str.as_bytes()).unwrap());
            assert_eq!(uuid_orig, uuid_out);
        }
    }

    #[test]
    fn test_try_parse_ascii_non_utf8() {
        assert!(Uuid::try_parse_ascii(b"67e55044-10b1-426f-9247-bb680e5\0e0c8").is_err());
    }
}
