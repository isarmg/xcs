#[test]
fn from_str_radix_multi_byte_fail() {
    // Ensure parsing doesn't panic, even on invalid sign characters
    assert!(f32::from_str_radix("™0.2", 10).is_err());

    // Even when parsing the exponent sign
    assert!(f32::from_str_radix("0.2E™1", 10).is_err());
}
