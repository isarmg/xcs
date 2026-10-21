#[test]
#[cfg_attr(miri, ignore)] // Miri is too slow
fn test_encode_utf8() {
    // Test that all codepoints are encoded correctly
    let mut data = [0u8; 16];
    for codepoint in 0..=(std::char::MAX as u32) {
        if let Some(ch) = std::char::from_u32(codepoint) {
            for elt in &mut data { *elt = 0; }
            let ptr = data.as_mut_ptr();
            let len = data.len();
            unsafe {
                let res = encode_utf8(ch, ptr, len).ok().unwrap();
                assert_eq!(res, ch.len_utf8());
            }
            let string = std::str::from_utf8(&data).unwrap();
            assert_eq!(string.chars().next(), Some(ch));
        }
    }
}
