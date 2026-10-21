#[test]
fn neon_code_matches_header_name_chars_table() {
    #[allow(clippy::undocumented_unsafe_blocks)]
    unsafe {
        assert!(byte_is_allowed(b'_', match_header_name_vectored));

        for (b, allowed) in crate::TOKEN_MAP.iter().cloned().enumerate() {
            assert_eq!(
                byte_is_allowed(b as u8, match_header_name_vectored),
                allowed,
                "byte_is_allowed({:?}) should be {:?}",
                b,
                allowed,
            );
        }
    }
}
