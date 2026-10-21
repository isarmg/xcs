#[cfg(test)]
mod tests {
    use super::{CodePointInversionList, InvalidSetError};
    use std::{char, vec::Vec};
    use zerovec::ZeroVec;

    #[test]
    fn test_codepointinversionlist_try_from_vec() {
        let ex = vec![0x2, 0x3, 0x4, 0x5];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(ex, check.get_inversion_list_vec());
        assert_eq!(2, check.size());
    }

    #[test]
    fn test_codepointinversionlist_try_from_vec_error() {
        let check = vec![0x1, 0x1, 0x2, 0x3, 0x4];
        let set = CodePointInversionList::try_from_u32_inversion_list_slice(&check);
        assert!(matches!(set, Err(InvalidSetError(_))));
        if let Err(InvalidSetError(actual)) = set {
            assert_eq!(
                &check,
                &actual.into_iter().map(u32::from).collect::<Vec<_>>()
            );
        }
    }

    // CodePointInversionList membership functions
    #[test]
    fn test_codepointinversionlist_contains_query() {
        let ex = vec![0x41, 0x46, 0x4B, 0x55];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert!(check.contains_query(0x40).is_none());
        assert_eq!(check.contains_query(0x41).unwrap(), 0);
        assert_eq!(check.contains_query(0x44).unwrap(), 0);
        assert!(check.contains_query(0x46).is_none());
        assert_eq!(check.contains_query(0x4C).unwrap(), 2);
        assert!(check.contains_query(0x56).is_none());
    }

    #[test]
    fn test_codepointinversionlist_contains() {
        let ex = vec![0x2, 0x5, 0xA, 0xF];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert!(check.contains(0x2 as char));
        assert!(check.contains(0x4 as char));
        assert!(check.contains(0xA as char));
        assert!(check.contains(0xE as char));
    }

    #[test]
    fn test_codepointinversionlist_contains_false() {
        let ex = vec![0x2, 0x5, 0xA, 0xF];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert!(!check.contains(0x1 as char));
        assert!(!check.contains(0x5 as char));
        assert!(!check.contains(0x9 as char));
        assert!(!check.contains(0xF as char));
        assert!(!check.contains(0x10 as char));
    }

    #[test]
    fn test_codepointinversionlist_contains_range() {
        let ex = vec![0x41, 0x46, 0x4B, 0x55];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert!(check.contains_range('A'..='E')); // 65 - 69
        assert!(check.contains_range('C'..'D')); // 67 - 67
        assert!(check.contains_range('L'..'P')); // 76 - 80
        assert!(!check.contains_range('L'..='U')); // 76 - 85
    }

    #[test]
    fn test_codepointinversionlist_contains_range_false() {
        let ex = vec![0x41, 0x46, 0x4B, 0x55];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert!(!check.contains_range('!'..'A')); // 33 - 65
        assert!(!check.contains_range('F'..'K')); // 70 - 74
        assert!(!check.contains_range('U'..)); // 85 - ..
    }

    #[test]
    fn test_codepointinversionlist_contains_range_invalid() {
        let check = CodePointInversionList::all();
        assert!(!check.contains_range('A'..'!')); // 65 - 33
        assert!(!check.contains_range('A'..'A')); // 65 - 65
    }

    #[test]
    fn test_codepointinversionlist_contains_set_u() {
        let ex = vec![0xA, 0x14, 0x28, 0x32, 0x46, 0x50, 0x64, 0x6E];
        let u = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        let inside = vec![0xF, 0x14, 0x2C, 0x31, 0x46, 0x50, 0x64, 0x6D];
        let s = CodePointInversionList::try_from_u32_inversion_list_slice(&inside).unwrap();
        assert!(u.contains_set(&s));
    }

    #[test]
    fn test_codepointinversionlist_contains_set_u_false() {
        let ex = vec![0xA, 0x14, 0x28, 0x32, 0x46, 0x50, 0x64, 0x78];
        let u = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        let outside = vec![0x0, 0xA, 0x16, 0x2C, 0x32, 0x46, 0x4F, 0x51, 0x6D, 0x6F];
        let s = CodePointInversionList::try_from_u32_inversion_list_slice(&outside).unwrap();
        assert!(!u.contains_set(&s));
    }

    #[test]
    fn test_codepointinversionlist_size() {
        let ex = vec![0x2, 0x5, 0xA, 0xF];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(8, check.size());
        let check = CodePointInversionList::all();
        let expected = (char::MAX as u32) + 1;
        assert_eq!(expected as usize, check.size());
        let inv_list_vec = vec![];
        let check = CodePointInversionList {
            inv_list: ZeroVec::from_slice_or_alloc(&inv_list_vec),
            size: 0,
        };
        assert_eq!(check.size(), 0);
    }

    #[test]
    fn test_codepointinversionlist_is_empty() {
        let inv_list_vec = vec![];
        let check = CodePointInversionList {
            inv_list: ZeroVec::from_slice_or_alloc(&inv_list_vec),
            size: 0,
        };
        assert!(check.is_empty());
    }

    #[test]
    fn test_codepointinversionlist_is_not_empty() {
        let check = CodePointInversionList::all();
        assert!(!check.is_empty());
    }

    #[test]
    fn test_codepointinversionlist_iter_chars() {
        let ex = vec![0x41, 0x44, 0x45, 0x46, 0xD800, 0xD801];
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        let mut iter = check.iter_chars();
        assert_eq!(Some('A'), iter.next());
        assert_eq!(Some('B'), iter.next());
        assert_eq!(Some('C'), iter.next());
        assert_eq!(Some('E'), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_codepointinversionlist_iter_ranges() {
        let ex = vec![0x41, 0x44, 0x45, 0x46, 0xD800, 0xD801];
        let set = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        let mut ranges = set.iter_ranges();
        assert_eq!(Some(0x41..=0x43), ranges.next());
        assert_eq!(Some(0x45..=0x45), ranges.next());
        assert_eq!(Some(0xD800..=0xD800), ranges.next());
        assert_eq!(None, ranges.next());
    }

    #[test]
    fn test_codepointinversionlist_iter_ranges_exactsizeiter_trait() {
        let ex = vec![0x41, 0x44, 0x45, 0x46, 0xD800, 0xD801];
        let set = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        let ranges = set.iter_ranges();
        assert_eq!(3, ranges.len());
    }

    #[test]
    fn test_codepointinversionlist_range_count() {
        let ex = vec![0x41, 0x44, 0x45, 0x46, 0xD800, 0xD801];
        let set = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(3, set.get_range_count());
    }

    #[test]
    fn test_codepointinversionlist_get_nth_range() {
        let ex = vec![0x41, 0x44, 0x45, 0x46, 0xD800, 0xD801];
        let set = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(Some(0x41..=0x43), set.get_nth_range(0));
        assert_eq!(Some(0x45..=0x45), set.get_nth_range(1));
        assert_eq!(Some(0xD800..=0xD800), set.get_nth_range(2));
        assert_eq!(None, set.get_nth_range(3));
    }

    // Range<char> cannot represent the upper bound (non-inclusive) for
    // char::MAX, whereas Range<u32> can.
    #[test]
    fn test_codepointinversionlist_iter_ranges_with_max_code_point() {
        let ex = vec![0x80, (char::MAX as u32) + 1];
        let set = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        let mut ranges = set.iter_ranges();
        assert_eq!(Some(0x80..=(char::MAX as u32)), ranges.next());
        assert_eq!(None, ranges.next());
    }

    #[test]
    fn test_codepointinversionlist_span_contains() {
        let ex = vec![0x41, 0x44, 0x46, 0x4B]; // A - D, F - K
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(check.span("ABCDE", true), 3);
        assert_eq!(check.span("E", true), 0);
    }

    #[test]
    fn test_codepointinversionlist_span_does_not_contain() {
        let ex = vec![0x41, 0x44, 0x46, 0x4B]; // A - D, F - K
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(check.span("DEF", false), 2);
        assert_eq!(check.span("KLMA", false), 3);
    }

    #[test]
    fn test_codepointinversionlist_span_back_contains() {
        let ex = vec![0x41, 0x44, 0x46, 0x4B]; // A - D, F - K
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(check.span_back("XYZABFH", true), 3);
        assert_eq!(check.span_back("ABCXYZ", true), 6);
    }

    #[test]
    fn test_codepointinversionlist_span_back_does_not_contain() {
        let ex = vec![0x41, 0x44, 0x46, 0x4B]; // A - D, F - K
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&ex).unwrap();
        assert_eq!(check.span_back("ABCXYZ", false), 3);
        assert_eq!(check.span_back("XYZABC", false), 6);
    }

    #[test]
    fn test_uniset_to_inv_list() {
        let inv_list = [
            0x9, 0xE, 0x20, 0x21, 0x85, 0x86, 0xA0, 0xA1, 0x1626, 0x1627, 0x2000, 0x2003, 0x2028,
            0x202A, 0x202F, 0x2030, 0x205F, 0x2060, 0x3000, 0x3001,
        ];
        let s: CodePointInversionList =
            CodePointInversionList::try_from_u32_inversion_list_slice(&inv_list).unwrap();
        let round_trip_inv_list = s.get_inversion_list_vec();
        assert_eq!(
            round_trip_inv_list.into_iter().collect::<ZeroVec<u32>>(),
            inv_list
        );
    }

    #[test]
    fn test_serialization_serialize() {
        let inv_list = [0x41, 0x46, 0x4B, 0x55];
        let uniset = CodePointInversionList::try_from_u32_inversion_list_slice(&inv_list).unwrap();
        let json_str = json_codec::to_string(&uniset).unwrap();
        assert_eq!(json_str, r#"["A-E","K-T"]"#);
    }

    #[test]
    fn test_serialization_serialize_surrogates() {
        let inv_list = [0xDFAB, 0xDFFF];
        let uniset = CodePointInversionList::try_from_u32_inversion_list_slice(&inv_list).unwrap();
        let json_str = json_codec::to_string(&uniset).unwrap();
        assert_eq!(json_str, r#"["U+DFAB-U+DFFE"]"#);
    }

    #[test]
    fn test_serialization_deserialize() {
        let inv_list_str = r#"["A-E","K-T"]"#;
        let exp_inv_list = [0x41, 0x46, 0x4B, 0x55];
        let exp_uniset =
            CodePointInversionList::try_from_u32_inversion_list_slice(&exp_inv_list).unwrap();
        let act_uniset: CodePointInversionList = json_codec::from_str(inv_list_str).unwrap();
        assert_eq!(act_uniset, exp_uniset);
    }

    #[test]
    fn test_serialization_deserialize_surrogates() {
        let inv_list_str = r#"["U+DFAB-U+DFFE"]"#;
        let exp_inv_list = [0xDFAB, 0xDFFF];
        let exp_uniset =
            CodePointInversionList::try_from_u32_inversion_list_slice(&exp_inv_list).unwrap();
        let act_uniset: CodePointInversionList = json_codec::from_str(inv_list_str).unwrap();
        assert_eq!(act_uniset, exp_uniset);
    }

    #[test]
    fn test_serialization_deserialize_invalid() {
        assert!(json_codec::from_str::<CodePointInversionList>("[65,70,98775,85]").is_err());
        assert!(json_codec::from_str::<CodePointInversionList>("[65,70,U+FFFFFFFFFF,85]").is_err());
    }

    #[test]
    fn test_serialization_with_postcard_roundtrip() -> Result<(), postcard::Error> {
        let set = CodePointInversionList::bmp();
        let set_serialized: Vec<u8> = postcard::to_allocvec(&set).unwrap();
        let set_deserialized: CodePointInversionList =
            postcard::from_bytes::<CodePointInversionList>(&set_serialized)?;

        assert_eq!(&set, &set_deserialized);
        assert!(!set_deserialized.inv_list.is_owned());

        Ok(())
    }

    #[test]
    fn databake() {
        databake::test_bake!(
            CodePointInversionList<'static>,
            const,
            unsafe {
                #[allow(unused_unsafe)]
                crate::codepointinvlist::CodePointInversionList::from_parts_unchecked(
                    unsafe {
                        zerovec::ZeroVec::from_bytes_unchecked(
                            b"0\0\0\0:\0\0\0A\0\0\0G\0\0\0a\0\0\0g\0\0\0",
                        )
                    },
                    22u32,
                )
            },
            icu_collections,
            [zerovec],
        );
    }
}
