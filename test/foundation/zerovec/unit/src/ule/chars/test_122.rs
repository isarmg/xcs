#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_array() {
        const CHARS: [char; 2] = ['a', '🙃'];
        const CHARS_ULE: [CharULE; 2] = CharULE::from_array(CHARS);
        assert_eq!(
            CharULE::slice_as_bytes(&CHARS_ULE),
            &[0x61, 0x00, 0x00, 0x43, 0xF6, 0x01]
        );
    }

    #[test]
    fn test_from_array_zst() {
        const CHARS: [char; 0] = [];
        const CHARS_ULE: [CharULE; 0] = CharULE::from_array(CHARS);
        let bytes = CharULE::slice_as_bytes(&CHARS_ULE);
        let empty: &[u8] = &[];
        assert_eq!(bytes, empty);
    }

    #[test]
    fn test_parse() {
        // 1-byte, 2-byte, 3-byte, and two 4-byte character in UTF-8 (not as relevant in UTF-32)
        let chars = ['w', 'ω', '文', '𑄃', '🙃'];
        let char_ules: Vec<CharULE> = chars.iter().copied().map(char::to_unaligned).collect();
        let char_bytes: &[u8] = CharULE::slice_as_bytes(&char_ules);

        // Check parsing
        let parsed_ules: &[CharULE] = CharULE::parse_bytes_to_slice(char_bytes).unwrap();
        assert_eq!(char_ules, parsed_ules);
        let parsed_chars: Vec<char> = parsed_ules
            .iter()
            .copied()
            .map(char::from_unaligned)
            .collect();
        assert_eq!(&chars, parsed_chars.as_slice());

        // Compare to golden expected data
        assert_eq!(
            &[119, 0, 0, 201, 3, 0, 135, 101, 0, 3, 17, 1, 67, 246, 1],
            char_bytes
        );
    }

    #[test]
    fn test_failures() {
        // 119 and 120 are valid, but not 0xD800 (high surrogate)
        let u32s = [119, 0xD800, 120];
        let u32_ules: Vec<RawBytesULE<4>> = u32s
            .iter()
            .copied()
            .map(<u32 as AsULE>::to_unaligned)
            .collect();
        let u32_bytes: &[u8] = RawBytesULE::<4>::slice_as_bytes(&u32_ules);
        let parsed_ules_result = CharULE::parse_bytes_to_slice(u32_bytes);
        assert!(parsed_ules_result.is_err());

        // 0x20FFFF is out of range for a char
        let u32s = [0x20FFFF];
        let u32_ules: Vec<RawBytesULE<4>> = u32s
            .iter()
            .copied()
            .map(<u32 as AsULE>::to_unaligned)
            .collect();
        let u32_bytes: &[u8] = RawBytesULE::<4>::slice_as_bytes(&u32_ules);
        let parsed_ules_result = CharULE::parse_bytes_to_slice(u32_bytes);
        assert!(parsed_ules_result.is_err());
    }
}
