#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_has_strings() {
        let cp_slice = &[0, 1, 0x7F, 0x80, 0xFFFF, 0x1_0000, 0x10_FFFF, 0x11_0000];
        let cp_list = CodePointInversionList::try_from_u32_inversion_list_slice(cp_slice).unwrap();
        let str_slice = &["ascii_max", "bmp_max", "unicode_max", "zero"];
        let str_list = VarZeroVec::<str>::from(str_slice);

        let cpilsl = CodePointInversionListAndStringList::try_from(cp_list, str_list).unwrap();

        assert!(cpilsl.has_strings());
        assert_eq!(8, cpilsl.size());
    }

    #[test]
    fn test_empty_string_allowed() {
        let cp_slice = &[0, 1, 0x7F, 0x80, 0xFFFF, 0x1_0000, 0x10_FFFF, 0x11_0000];
        let cp_list = CodePointInversionList::try_from_u32_inversion_list_slice(cp_slice).unwrap();
        let str_slice = &["", "ascii_max", "bmp_max", "unicode_max", "zero"];
        let str_list = VarZeroVec::<str>::from(str_slice);

        let cpilsl = CodePointInversionListAndStringList::try_from(cp_list, str_list).unwrap();

        assert!(cpilsl.has_strings());
        assert_eq!(9, cpilsl.size());
    }

    #[test]
    fn test_invalid_string() {
        let cp_slice = &[0, 1];
        let cp_list = CodePointInversionList::try_from_u32_inversion_list_slice(cp_slice).unwrap();
        let str_slice = &["a"];
        let str_list = VarZeroVec::<str>::from(str_slice);

        let cpilsl = CodePointInversionListAndStringList::try_from(cp_list, str_list);

        assert!(matches!(
            cpilsl,
            Err(InvalidStringList::InvalidStringLength(_))
        ));
    }

    #[test]
    fn test_invalid_string_list_has_duplicate() {
        let cp_slice = &[0, 1];
        let cp_list = CodePointInversionList::try_from_u32_inversion_list_slice(cp_slice).unwrap();
        let str_slice = &["abc", "abc"];
        let str_list = VarZeroVec::<str>::from(str_slice);

        let cpilsl = CodePointInversionListAndStringList::try_from(cp_list, str_list);

        assert!(matches!(
            cpilsl,
            Err(InvalidStringList::StringListNotUnique(_))
        ));
    }

    #[test]
    fn test_invalid_string_list_not_sorted() {
        let cp_slice = &[0, 1];
        let cp_list = CodePointInversionList::try_from_u32_inversion_list_slice(cp_slice).unwrap();
        let str_slice = &["xyz", "abc"];
        let str_list = VarZeroVec::<str>::from(str_slice);

        let cpilsl = CodePointInversionListAndStringList::try_from(cp_list, str_list);

        assert!(matches!(
            cpilsl,
            Err(InvalidStringList::StringListNotSorted(_, _))
        ));
    }

    #[test]
    fn test_from_iter_invariants() {
        let in_strs_1 = ["a", "abc", "xyz", "abc"];
        let in_strs_2 = ["xyz", "abc", "a", "abc"];

        let cpilsl_1 = CodePointInversionListAndStringList::from_iter(in_strs_1);
        let cpilsl_2 = CodePointInversionListAndStringList::from_iter(in_strs_2);

        assert_eq!(cpilsl_1, cpilsl_2);

        assert!(cpilsl_1.has_strings());
        assert!(cpilsl_1.contains_str("abc"));
        assert!(cpilsl_1.contains_str("xyz"));
        assert!(!cpilsl_1.contains_str("def"));

        assert_eq!(1, cpilsl_1.cp_inv_list.size());
        assert!(cpilsl_1.contains('a'));
        assert!(!cpilsl_1.contains('0'));
        assert!(!cpilsl_1.contains('q'));

        assert_eq!(3, cpilsl_1.size());
    }
}
