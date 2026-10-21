#[test]
fn neon_code_matches_uri_chars_table() {
    #[allow(clippy::undocumented_unsafe_blocks)]
    unsafe {
        assert!(byte_is_allowed(b'_', match_uri_vectored));

        for (b, allowed) in crate::URI_MAP.iter().cloned().enumerate() {
            assert_eq!(
                byte_is_allowed(b as u8, match_uri_vectored),
                allowed,
                "byte_is_allowed({:?}) should be {:?}",
                b,
                allowed,
            );
        }
    }
}
