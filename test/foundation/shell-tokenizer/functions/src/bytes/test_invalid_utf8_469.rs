#[test]
#[allow(invalid_from_utf8)]
fn test_invalid_utf8() {
    // Check that our test string is actually invalid UTF-8.
    assert!(core::str::from_utf8(INVALID_UTF8).is_err());
}
