#[cfg(test)]
mod tests {
    use super::{CodePointInversionList, CodePointInversionListBuilder};

    fn generate_tester(ex: &[u32]) -> CodePointInversionListBuilder {
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(ex).unwrap();
        let mut builder = CodePointInversionListBuilder::new();
        builder.add_set(&check);
        builder
    }

    #[test]
    fn test_new() {
        let ex = CodePointInversionListBuilder::new();
        assert!(ex.intervals.is_empty());
    }

    #[test]
    fn test_build() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add(0x41, 0x42);
        let check: CodePointInversionList = builder.build();
        assert_eq!(check.iter_chars().next(), Some('A'));
    }

    #[test]
    fn test_empty_build() {
        let builder = CodePointInversionListBuilder::new();
        let check: CodePointInversionList = builder.build();
        assert!(check.is_empty());
    }

    #[test]
    fn test_add_to_empty() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add(0x0, 0xA);
        assert_eq!(builder.intervals, [0x0, 0xA]);
    }

    #[test]
    fn test_add_invalid() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add(0x0, 0x0);
        builder.add(0x5, 0x0);
        assert!(builder.intervals.is_empty());
    }

    #[test]
    fn test_add_to_start() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x0, 0x5);
        let expected = [0x0, 0x5, 0xA, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_start_overlap() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x0, 0xE);
        let expected = [0x0, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_end() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x3C, 0x46);
        let expected = [0xA, 0x14, 0x28, 0x32, 60, 70];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_end_overlap() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x2B, 0x46);
        let expected = [0xA, 0x14, 0x28, 0x46];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_middle_no_overlap() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x19, 0x1B);
        let expected = [0xA, 0x14, 0x19, 0x1B, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_middle_inside() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0xA, 0x14);
        let expected = [0xA, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_middle_left_overlap() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0xF, 0x19);
        let expected = [0xA, 0x19, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_middle_right_overlap() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x1E, 0x28);
        let expected = [0xA, 0x14, 0x1E, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_full_encompass() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x0, 0x3C);
        let expected = [0x0, 0x3C];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_to_partial_encompass() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x0, 0x23);
        let expected = [0x0, 0x23, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_aligned_front() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(5, 10);
        let expected = [5, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_aligned_back() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x32, 0x37);
        let expected = [0xA, 0x14, 0x28, 0x37];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_aligned_start_middle() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x14, 0x19);
        let expected = [0xA, 0x19, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_aligned_end_middle() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x23, 0x28);
        let expected = [0xA, 0x14, 0x23, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_aligned_in_between_end() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x1E, 0x28, 0x32, 0x3C]);
        builder.add(0xF, 0x1E);
        let expected = [0xA, 0x28, 0x32, 0x3C];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_aligned_in_between_start() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x1E, 0x28, 0x32, 0x3C]);
        builder.add(20, 35);
        let expected = [0xA, 0x28, 0x32, 0x3C];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_adjacent_ranges() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.add(0x13, 0x14);
        builder.add(0x14, 0x15);
        builder.add(0x15, 0x16);
        let expected = [0xA, 0x16, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_codepointinversionlist() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        let check = CodePointInversionList::try_from_u32_inversion_list_slice(&[
            0x5, 0xA, 0x16, 0x21, 0x2C, 0x33,
        ])
        .unwrap();
        builder.add_set(&check);
        let expected = [0x5, 0x14, 0x16, 0x21, 0x28, 0x33];
        assert_eq!(builder.intervals, expected);
    }
    #[test]
    fn test_add_char() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add_char('a');
        let expected = [0x61, 0x62];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_range() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add_range('A'..='Z');
        let expected = [0x41, 0x5B];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_range32() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add_range32(0xd800..=0xdfff);
        let expected = [0xd800, 0xe000];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_add_invalid_range() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.add_range('Z'..='A');
        assert!(builder.intervals.is_empty());
    }

    #[test]
    fn test_remove_empty() {
        let mut builder = CodePointInversionListBuilder::new();
        builder.remove(0x0, 0xA);
        assert!(builder.intervals.is_empty());
    }

    #[test]
    fn test_remove_entire_builder() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0xA, 0x32);
        assert!(builder.intervals.is_empty());
    }

    #[test]
    fn test_remove_entire_range() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0xA, 0x14);
        let expected = [0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_partial_range_left() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0xA, 0x2B);
        let expected = [0x2B, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_ne_range() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0x14, 0x28);
        let expected = [0xA, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_partial_range_right() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0xF, 0x37);
        let expected = [0xA, 0xF];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_middle_range() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0xC, 0x12);
        let expected = [0xA, 0xC, 0x12, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_ne_middle_range() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0x19, 0x1B);
        let expected = [0xA, 0x14, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_encompassed_range() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32, 70, 80]);
        builder.remove(0x19, 0x37);
        let expected = [0xA, 0x14, 0x46, 0x50];
        assert_eq!(builder.intervals, expected);
    }
    #[test]
    fn test_remove_adjacent_ranges() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.remove(0x27, 0x28);
        builder.remove(0x28, 0x29);
        builder.remove(0x29, 0x2A);
        let expected = [0xA, 0x14, 0x2A, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_char() {
        let mut builder = generate_tester(&[0x41, 0x46]);
        builder.remove_char('A'); // 65
        let expected = [0x42, 0x46];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_range() {
        let mut builder = generate_tester(&[0x41, 0x5A]);
        builder.remove_range('A'..'L'); // 65 - 76
        let expected = [0x4C, 0x5A];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_remove_set() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32, 70, 80]);
        let remove =
            CodePointInversionList::try_from_u32_inversion_list_slice(&[0xA, 0x14, 0x2D, 0x4B])
                .unwrap();
        builder.remove_set(&remove);
        let expected = [0x28, 0x2D, 0x4B, 0x50];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_retain_char() {
        let mut builder = generate_tester(&[0x41, 0x5A]);
        builder.retain_char('A'); // 65
        let expected = [0x41, 0x42];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_retain_range() {
        let mut builder = generate_tester(&[0x41, 0x5A]);
        builder.retain_range('C'..'F'); // 67 - 70
        let expected = [0x43, 0x46];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_retain_range_empty() {
        let mut builder = generate_tester(&[0x41, 0x46]);
        builder.retain_range('F'..'Z');
        assert!(builder.intervals.is_empty());
    }

    #[test]
    fn test_retain_set() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32, 70, 80]);
        let retain = CodePointInversionList::try_from_u32_inversion_list_slice(&[
            0xE, 0x14, 0x19, 0x37, 0x4D, 0x51,
        ])
        .unwrap();
        builder.retain_set(&retain);
        let expected = [0xE, 0x14, 0x28, 0x32, 0x4D, 0x50];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.complement();
        let expected = [0x0, 0xA, 0x14, 0x28, 0x32, (char::MAX as u32) + 1];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_empty() {
        let mut builder = generate_tester(&[]);
        builder.complement();
        let expected = [0x0, (char::MAX as u32) + 1];
        assert_eq!(builder.intervals, expected);

        builder.complement();
        let expected: [u32; 0] = [];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_zero_max() {
        let mut builder = generate_tester(&[0x0, 0xA, 0x5A, (char::MAX as u32) + 1]);
        builder.complement();
        let expected = [0xA, 0x5A];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_interior() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.complement_list([0xE, 0x14]);
        let expected = [0xA, 0xE, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_exterior() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.complement_list([0x19, 0x23]);
        let expected = [0xA, 0x14, 0x19, 0x23, 0x28, 0x32];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_larger_list() {
        let mut builder = generate_tester(&[0xA, 0x14, 0x28, 0x32]);
        builder.complement_list([0x1E, 0x37, 0x3C, 0x46]);
        let expected = [0xA, 0x14, 0x1E, 0x28, 0x32, 0x37, 0x3C, 0x46];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_char() {
        let mut builder = generate_tester(&[0x41, 0x4C]); // A - K
        builder.complement_char('A');
        builder.complement_char('L');
        let expected = [0x42, 0x4D];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_range() {
        let mut builder = generate_tester(&[0x46, 0x4C]); // F - K
        builder.complement_range('A'..='Z');
        let expected = [0x41, 0x46, 0x4C, 0x5B];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_complement_set() {
        let mut builder = generate_tester(&[0x43, 0x4E]);
        let set =
            CodePointInversionList::try_from_u32_inversion_list_slice(&[0x41, 0x46, 0x4B, 0x5A])
                .unwrap();
        builder.complement_set(&set);
        let expected = [0x41, 0x43, 0x46, 0x4B, 0x4E, 0x5A];
        assert_eq!(builder.intervals, expected);
    }

    #[test]
    fn test_is_empty() {
        let builder = CodePointInversionListBuilder::new();
        assert!(builder.is_empty());
    }
}
