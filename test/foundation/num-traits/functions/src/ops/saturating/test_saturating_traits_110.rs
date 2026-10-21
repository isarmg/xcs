#[test]
fn test_saturating_traits() {
    fn saturating_add<T: SaturatingAdd>(a: T, b: T) -> T {
        a.saturating_add(&b)
    }
    fn saturating_sub<T: SaturatingSub>(a: T, b: T) -> T {
        a.saturating_sub(&b)
    }
    fn saturating_mul<T: SaturatingMul>(a: T, b: T) -> T {
        a.saturating_mul(&b)
    }
    assert_eq!(saturating_add(255, 1), 255u8);
    assert_eq!(saturating_add(127, 1), 127i8);
    assert_eq!(saturating_add(-128, -1), -128i8);
    assert_eq!(saturating_sub(0, 1), 0u8);
    assert_eq!(saturating_sub(-128, 1), -128i8);
    assert_eq!(saturating_sub(127, -1), 127i8);
    assert_eq!(saturating_mul(255, 2), 255u8);
    assert_eq!(saturating_mul(127, 2), 127i8);
    assert_eq!(saturating_mul(-128, 2), -128i8);
}
