#[test]
fn from_str_radix_ignore_case() {
    assert_eq!(
        f32::from_str_radix("InF", 16).unwrap(),
        f32::INFINITY
    );
    assert_eq!(
        f32::from_str_radix("InfinitY", 16).unwrap(),
        f32::INFINITY
    );
    assert_eq!(
        f32::from_str_radix("-InF", 8).unwrap(),
        f32::NEG_INFINITY
    );
    assert_eq!(
        f32::from_str_radix("-InfinitY", 8).unwrap(),
        f32::NEG_INFINITY
    );
    assert!(f32::from_str_radix("nAn", 4).unwrap().is_nan());
    assert!(f32::from_str_radix("-nAn", 4).unwrap().is_nan());
}
