#[test]
#[allow(warnings)] // ... ranges deprecated
fn test_lookup_tables() {
    for (i, &valid) in TOKEN_MAP.iter().enumerate() {
        let i = i as u8;
        let should = match i {
            b'a'...b'z' |
            b'A'...b'Z' |
            b'0'...b'9' |
            b'!' |
            b'#' |
            b'$' |
            b'%' |
            b'&' |
            b'\'' |
            b'*' |
            b'+' |
            b'-' |
            b'.' |
            b'^' |
            b'_' |
            b'`' |
            b'|' |
            b'~' => true,
            _ => false
        };
        assert_eq!(valid, should, "{:?} ({}) should be {}", i as char, i, should);
    }
}
