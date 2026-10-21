#[cfg(test)]
mod tests {
    use super::*;
    use crate::ZeroSlice;

    #[test]
    fn test_array_ule_validate() {
        let bytes: &[u8] = &[1, 2, 3, 4, 5, 6];
        assert!(<[u8; 2] as ULE>::validate_bytes(bytes).is_ok());
        assert!(<[u8; 3] as ULE>::validate_bytes(bytes).is_ok());
        assert!(<[u8; 6] as ULE>::validate_bytes(bytes).is_ok());

        // Length not a multiple of array size
        assert!(<[u8; 4] as ULE>::validate_bytes(bytes).is_err());
        assert!(<[u8; 5] as ULE>::validate_bytes(bytes).is_err());
        assert!(<[u8; 7] as ULE>::validate_bytes(bytes).is_err());

        // Multi-byte element types (CharULE is 3 bytes, [CharULE; 2] is 6 bytes)
        let chars_6b: &[u8] = &[0x61, 0x00, 0x00, 0x62, 0x00, 0x00]; // 'a', 'b'
        assert!(<[CharULE; 2] as ULE>::validate_bytes(chars_6b).is_ok());
        let chars_9b: &[u8] = &[0x61, 0x00, 0x00, 0x62, 0x00, 0x00, 0x63, 0x00, 0x00]; // 'a', 'b', 'c' (9 bytes: multiple of CharULE (3), but not [CharULE; 2] (6))
        assert!(<[CharULE; 2] as ULE>::validate_bytes(chars_9b).is_err());

        // ZeroSlice::parse_bytes
        assert!(ZeroSlice::<[u8; 3]>::parse_bytes(bytes).is_ok());
        assert!(ZeroSlice::<[u8; 4]>::parse_bytes(bytes).is_err());

        // Zero-length arrays unconditionally error
        assert!(<[u8; 0] as ULE>::validate_bytes(&[]).is_err());
        assert!(<[u8; 0] as ULE>::validate_bytes(bytes).is_err());
        assert!(ZeroSlice::<[u8; 0]>::parse_bytes(&[]).is_err());
        assert!(ZeroSlice::<[u8; 0]>::parse_bytes(bytes).is_err());
    }
}
