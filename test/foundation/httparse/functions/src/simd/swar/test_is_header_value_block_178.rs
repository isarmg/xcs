#[test]
fn test_is_header_value_block() {
    let is_header_value_block = |b| match_header_value_char_8_swar(b) == BLOCK_SIZE;

    // 0..32 => false
    for b in 0..32_u8 {
        assert!(!is_header_value_block([b; BLOCK_SIZE]), "b={}", b);
    }
    // 32..=126 => true
    for b in 32..=126_u8 {
        assert!(is_header_value_block([b; BLOCK_SIZE]), "b={}", b);
    }
    // 127 => false
    assert!(!is_header_value_block([b'\x7F'; BLOCK_SIZE]), "b={}", b'\x7F');
    // 128..=255 => true
    for b in 128..=255_u8 {
        assert!(is_header_value_block([b; BLOCK_SIZE]), "b={}", b);
    }


    #[cfg(target_pointer_width = "64")]
    {
        // A few sanity checks on non-uniform bytes for safe-measure
        assert!(!is_header_value_block(*b"foo.com\n"));
        assert!(!is_header_value_block(*b"o.com\r\nU"));
    }
}
