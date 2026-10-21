#[test]
fn test_is_uri_block() {
    let is_uri_block = |b| match_uri_char_8_swar(b) == BLOCK_SIZE;

    // 0..33 => false
    for b in 0..33_u8 {
        assert!(!is_uri_block([b; BLOCK_SIZE]), "b={}", b);
    }
    // 33..=126 => true
    for b in 33..=126_u8 {
        assert!(is_uri_block([b; BLOCK_SIZE]), "b={}", b);
    }
    // 127 => false
    assert!(!is_uri_block([b'\x7F'; BLOCK_SIZE]), "b={}", b'\x7F');
    // 128..=255 => true
    for b in 128..=255_u8 {
        assert!(is_uri_block([b; BLOCK_SIZE]), "b={}", b);
    }
}
