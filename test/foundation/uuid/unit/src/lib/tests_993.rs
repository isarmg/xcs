#[cfg(test)]
mod tests {
    use super::*;

    use crate::std::string::{String, ToString};

    #[cfg(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")))]
    use wasm_bindgen_test::*;

    macro_rules! check {
        ($buf:ident, $format:expr, $target:expr, $len:expr, $cond:expr) => {
            $buf.clear();
            write!($buf, $format, $target).unwrap();
            assert!($buf.len() == $len);
            assert!($buf.chars().all($cond), "{}", $buf);

            assert_eq!(Uuid::parse_str(&$buf).unwrap(), $target);
        };
    }

    pub fn some_uuid_nil() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
    }

    pub fn some_uuid_v1() -> Uuid {
        Uuid::parse_str("20616934-4ba2-11e7-8000-010203040506").unwrap()
    }

    pub fn some_uuid_v3() -> Uuid {
        Uuid::parse_str("bcee7a9c-52f1-30c6-a3cc-8c72ba634990").unwrap()
    }

    pub fn some_uuid_v4() -> Uuid {
        Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap()
    }

    pub fn some_uuid_v4_2() -> Uuid {
        Uuid::parse_str("c0dd0820-b35a-4c56-bc7d-0f0b04241adb").unwrap()
    }

    pub fn some_uuid_v5() -> Uuid {
        Uuid::parse_str("b11f79a5-1e6d-57ce-a4b5-ba8531ea03d0").unwrap()
    }

    pub fn some_uuid_v6() -> Uuid {
        Uuid::parse_str("1e74ba22-0616-6934-8000-010203040506").unwrap()
    }

    pub fn some_uuid_v7() -> Uuid {
        Uuid::parse_str("015c837b-9e84-7db5-b059-c75a84585688").unwrap()
    }

    pub fn some_uuid_v8() -> Uuid {
        Uuid::parse_str("0f0e0d0c-0b0a-8908-8706-050403020100").unwrap()
    }

    pub fn some_uuid_max() -> Uuid {
        Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap()
    }

    pub fn some_uuid_iter() -> impl Iterator<Item = Uuid> {
        [
            some_uuid_nil(),
            some_uuid_v1(),
            some_uuid_v3(),
            some_uuid_v4(),
            some_uuid_v5(),
            some_uuid_v6(),
            some_uuid_v7(),
            some_uuid_v8(),
            some_uuid_max(),
        ]
        .into_iter()
    }

    pub fn some_uuid_v_iter() -> impl Iterator<Item = Uuid> {
        [
            some_uuid_v1(),
            some_uuid_v3(),
            some_uuid_v4(),
            some_uuid_v5(),
            some_uuid_v6(),
            some_uuid_v7(),
            some_uuid_v8(),
        ]
        .into_iter()
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    #[cfg(feature = "std")]
    fn test_compare() {
        use std::{
            cmp::Ordering,
            hash::{BuildHasher, BuildHasherDefault, DefaultHasher},
        };

        let a = some_uuid_v4();
        let b = some_uuid_v4_2();

        let ah = BuildHasherDefault::<DefaultHasher>::default().hash_one(a);
        let bh = BuildHasherDefault::<DefaultHasher>::default().hash_one(b);

        assert_eq!(a, a);
        assert_eq!(b, b);
        assert_eq!(Ordering::Equal, a.cmp(&a));
        assert_eq!(Ordering::Equal, b.cmp(&b));

        assert_ne!(a, b);
        assert_ne!(b, a);
        assert_ne!(Ordering::Equal, b.cmp(&a));
        assert_ne!(Ordering::Equal, a.cmp(&b));
        assert_ne!(ah, bh);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_default() {
        let default_uuid = Uuid::default();
        let nil_uuid = Uuid::nil();

        assert_eq!(default_uuid, nil_uuid);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_display() {
        use crate::std::fmt::Write;

        for uuid in some_uuid_iter() {
            let s = uuid.to_string();
            let mut buffer = String::new();

            assert_eq!(s, uuid.hyphenated().to_string());

            check!(buffer, "{}", some_uuid_v4(), 36, |c| c.is_lowercase()
                || c.is_ascii_digit()
                || c == '-');
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_to_simple_string() {
        for uuid in some_uuid_iter() {
            let s = uuid.simple().to_string();

            assert_eq!(s.len(), 32);
            assert!(s.chars().all(|c| c.is_ascii_hexdigit()));

            assert_eq!(Uuid::parse_str(&s).unwrap(), uuid);
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_hyphenated_string() {
        for uuid in some_uuid_iter() {
            let s = uuid.hyphenated().to_string();

            assert_eq!(36, s.len());
            assert!(s.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));

            assert_eq!(Uuid::parse_str(&s).unwrap(), uuid);
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_upper_lower_hex() {
        use std::fmt::Write;

        let mut buf = String::new();

        macro_rules! check {
            ($buf:ident, $format:expr, $target:expr, $len:expr, $cond:expr) => {
                $buf.clear();
                write!($buf, $format, $target).unwrap();
                assert_eq!($len, buf.len());
                assert!($buf.chars().all($cond), "{}", $buf);
            };
        }

        for uuid in some_uuid_iter() {
            check!(buf, "{:x}", uuid, 36, |c| c.is_lowercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:X}", uuid, 36, |c| c.is_uppercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:#x}", uuid, 36, |c| c.is_lowercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:#X}", uuid, 36, |c| c.is_uppercase()
                || c.is_ascii_digit()
                || c == '-');

            check!(buf, "{:X}", uuid.hyphenated(), 36, |c| c.is_uppercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:X}", uuid.simple(), 32, |c| c.is_uppercase()
                || c.is_ascii_digit());
            check!(buf, "{:#X}", uuid.hyphenated(), 36, |c| c.is_uppercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:#X}", uuid.simple(), 32, |c| c.is_uppercase()
                || c.is_ascii_digit());

            check!(buf, "{:x}", uuid.hyphenated(), 36, |c| c.is_lowercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:x}", uuid.simple(), 32, |c| c.is_lowercase()
                || c.is_ascii_digit());
            check!(buf, "{:#x}", uuid.hyphenated(), 36, |c| c.is_lowercase()
                || c.is_ascii_digit()
                || c == '-');
            check!(buf, "{:#x}", uuid.simple(), 32, |c| c.is_lowercase()
                || c.is_ascii_digit());
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_to_urn_string() {
        for uuid in some_uuid_iter() {
            let ss = uuid.urn().to_string();
            let s = &ss[9..];

            assert!(ss.starts_with("urn:uuid:"));
            assert_eq!(s.len(), 36);
            assert!(s.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));

            assert_eq!(Uuid::parse_str(&ss).unwrap(), uuid);
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_nil() {
        let nil = Uuid::nil();
        let not_nil = some_uuid_v4();

        assert!(nil.is_nil());
        assert!(!not_nil.is_nil());

        assert_eq!(nil.get_version(), Some(Version::Nil));
        assert_eq!(nil.get_variant(), Variant::NCS);

        assert_eq!(not_nil.get_version(), Some(Version::Random));

        assert_eq!(
            nil,
            Builder::from_bytes([0; 16])
                .with_version(Version::Nil)
                .into_uuid()
        );
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_max() {
        let max = Uuid::max();
        let not_max = some_uuid_v4();

        assert!(max.is_max());
        assert!(!not_max.is_max());

        assert_eq!(max.get_version(), Some(Version::Max));
        assert_eq!(max.get_variant(), Variant::Future);

        assert_eq!(not_max.get_version(), Some(Version::Random));

        assert_eq!(
            max,
            Builder::from_bytes([0xff; 16])
                .with_version(Version::Max)
                .into_uuid()
        );
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_predefined_namespaces() {
        assert_eq!(
            Uuid::NAMESPACE_DNS.hyphenated().to_string(),
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        );
        assert_eq!(
            Uuid::NAMESPACE_URL.hyphenated().to_string(),
            "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
        );
        assert_eq!(
            Uuid::NAMESPACE_OID.hyphenated().to_string(),
            "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
        );
        assert_eq!(
            Uuid::NAMESPACE_X500.hyphenated().to_string(),
            "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_get_timestamp_unsupported_version() {
        for uuid in [
            some_uuid_nil(),
            some_uuid_v3(),
            some_uuid_v4(),
            some_uuid_v5(),
            some_uuid_v8(),
            some_uuid_max(),
        ] {
            assert_ne!(Version::Mac, uuid.get_version().unwrap());
            assert_ne!(Version::SortMac, uuid.get_version().unwrap());
            assert_ne!(Version::SortRand, uuid.get_version().unwrap());

            assert!(uuid.get_timestamp().is_none());
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_get_node_id_unsupported_version() {
        for uuid in [
            some_uuid_nil(),
            some_uuid_v4(),
            some_uuid_v7(),
            some_uuid_v8(),
            some_uuid_max(),
        ] {
            assert_ne!(Version::Mac, uuid.get_version().unwrap());
            assert_ne!(Version::SortMac, uuid.get_version().unwrap());

            assert!(uuid.get_node_id().is_none());
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_get_version() {
        fn assert_version(uuid: Uuid, expected: Version) {
            assert_eq!(
                uuid.get_version().unwrap(),
                expected,
                "{uuid} version doesn't match {expected:?}"
            );
            assert_eq!(
                uuid.get_version_num(),
                expected as usize,
                "{uuid} version doesn't match {}",
                expected as usize
            );
        }

        assert_version(some_uuid_nil(), Version::Nil);
        assert_version(some_uuid_v1(), Version::Mac);
        assert_version(some_uuid_v3(), Version::Md5);
        assert_version(some_uuid_v4(), Version::Random);
        assert_version(some_uuid_v5(), Version::Sha1);
        assert_version(some_uuid_v6(), Version::SortMac);
        assert_version(some_uuid_v7(), Version::SortRand);
        assert_version(some_uuid_v8(), Version::Custom);
        assert_version(some_uuid_max(), Version::Max);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_get_version_non_conforming() {
        for case in [
            Uuid::from_bytes([4, 54, 67, 12, 43, 2, 2, 76, 32, 50, 87, 5, 1, 33, 43, 87]),
            Uuid::parse_str("00000000-0000-0000-0000-00000000000f").unwrap(),
            Uuid::parse_str("ffffffff-ffff-ffff-ffff-fffffffffff0").unwrap(),
        ] {
            assert_eq!(case.get_version(), None);
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_get_variant() {
        fn assert_variant(uuid: Uuid, expected: Variant) {
            assert_eq!(uuid.get_variant(), expected);
        }

        for uuid in some_uuid_v_iter() {
            assert_variant(uuid, Variant::RFC4122);
        }

        assert_variant(
            Uuid::parse_str("936DA01F9ABD4d9dC0C702AF85C822A8").unwrap(),
            Variant::Microsoft,
        );
        assert_variant(
            Uuid::parse_str("F9168C5E-CEB2-4faa-D6BF-329BF39FA1E4").unwrap(),
            Variant::Microsoft,
        );
        assert_variant(
            Uuid::parse_str("f81d4fae-7dec-11d0-7765-00a0c91e6bf6").unwrap(),
            Variant::NCS,
        );
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_from_fields() {
        let d1: u32 = 0xa1a2a3a4;
        let d2: u16 = 0xb1b2;
        let d3: u16 = 0xc1c2;
        let d4 = [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];

        let u = Uuid::from_fields(d1, d2, d3, &d4);

        let expected = "a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8";
        let result = u.simple().to_string();
        assert_eq!(result, expected);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_from_fields_le() {
        let d1: u32 = 0xa4a3a2a1;
        let d2: u16 = 0xb2b1;
        let d3: u16 = 0xc2c1;
        let d4 = [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];

        let u = Uuid::from_fields_le(d1, d2, d3, &d4);

        let expected = "a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8";
        let result = u.simple().to_string();
        assert_eq!(result, expected);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_fields_roundtrip() {
        let d1_in: u32 = 0xa1a2a3a4;
        let d2_in: u16 = 0xb1b2;
        let d3_in: u16 = 0xc1c2;
        let d4_in = &[0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];

        let u = Uuid::from_fields(d1_in, d2_in, d3_in, d4_in);
        let (d1_out, d2_out, d3_out, d4_out) = u.as_fields();

        assert_eq!(d1_in, d1_out);
        assert_eq!(d2_in, d2_out);
        assert_eq!(d3_in, d3_out);
        assert_eq!(d4_in, d4_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_fields_le_roundtrip() {
        let d1_in: u32 = 0xa4a3a2a1;
        let d2_in: u16 = 0xb2b1;
        let d3_in: u16 = 0xc2c1;
        let d4_in = &[0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];

        let u = Uuid::from_fields_le(d1_in, d2_in, d3_in, d4_in);
        let (d1_out, d2_out, d3_out, d4_out) = u.to_fields_le();

        assert_eq!(d1_in, d1_out);
        assert_eq!(d2_in, d2_out);
        assert_eq!(d3_in, d3_out);
        assert_eq!(d4_in, d4_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_fields_le_are_actually_le() {
        let d1_in: u32 = 0xa1a2a3a4;
        let d2_in: u16 = 0xb1b2;
        let d3_in: u16 = 0xc1c2;
        let d4_in = &[0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8];

        let u = Uuid::from_fields(d1_in, d2_in, d3_in, d4_in);
        let (d1_out, d2_out, d3_out, d4_out) = u.to_fields_le();

        assert_eq!(d1_in, d1_out.swap_bytes());
        assert_eq!(d2_in, d2_out.swap_bytes());
        assert_eq!(d3_in, d3_out.swap_bytes());
        assert_eq!(d4_in, d4_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_u128_roundtrip() {
        let v_in: u128 = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8;

        let u = Uuid::from_u128(v_in);
        let v_out = u.as_u128();

        assert_eq!(v_in, v_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_u128_le_roundtrip() {
        let v_in: u128 = 0xd8d7d6d5d4d3d2d1c2c1b2b1a4a3a2a1;

        let u = Uuid::from_u128_le(v_in);
        let v_out = u.to_u128_le();

        assert_eq!(v_in, v_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_u128_le_is_actually_le() {
        let v_in: u128 = 0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8;

        let u = Uuid::from_u128(v_in);
        let v_out = u.to_u128_le();

        assert_eq!(v_in, v_out.swap_bytes());
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_u64_pair_roundtrip() {
        let high_in: u64 = 0xa1a2a3a4b1b2c1c2;
        let low_in: u64 = 0xd1d2d3d4d5d6d7d8;

        let u = Uuid::from_u64_pair(high_in, low_in);
        let (high_out, low_out) = u.as_u64_pair();

        assert_eq!(high_in, high_out);
        assert_eq!(low_in, low_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_from_slice() {
        let b = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];

        let u = Uuid::from_slice(&b).unwrap();
        let expected = "a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8";

        assert_eq!(u.simple().to_string(), expected);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_from_bytes() {
        let b = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];

        let u = Uuid::from_bytes(b);
        let expected = "a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8";

        assert_eq!(u.simple().to_string(), expected);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_as_bytes() {
        for uuid in some_uuid_v_iter() {
            let ub = uuid.as_bytes();
            let ur: &[u8] = uuid.as_ref();

            assert_eq!(ub.len(), 16);
            assert_eq!(ur.len(), 16);
            assert!(!ub.iter().all(|&b| b == 0));
            assert!(!ur.iter().all(|&b| b == 0));
        }
    }

    #[test]
    #[cfg(feature = "std")]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_convert_vec() {
        for uuid in some_uuid_iter() {
            let ub: &[u8] = uuid.as_ref();

            let v: std::vec::Vec<u8> = uuid.into();

            assert_eq!(&v, ub);

            let uv: Uuid = v.try_into().unwrap();

            assert_eq!(uv, uuid);
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_bytes_roundtrip() {
        let b_in: crate::Bytes = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];

        let u = Uuid::from_slice(&b_in).unwrap();

        let b_out = u.as_bytes();

        assert_eq!(&b_in, b_out);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_bytes_le_roundtrip() {
        let b = [
            0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8,
        ];

        let u1 = Uuid::from_bytes(b);

        let b_le = u1.to_bytes_le();

        let u2 = Uuid::from_bytes_le(b_le);

        assert_eq!(u1, u2);
    }
}
