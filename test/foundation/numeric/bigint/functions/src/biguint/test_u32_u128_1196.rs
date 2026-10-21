#[test]
fn test_u32_u128() {
    assert_eq!(u32_from_u128(0u128), (0, 0, 0, 0));
    assert_eq!(
        u32_from_u128(u128::MAX),
        (u32::MAX, u32::MAX, u32::MAX, u32::MAX)
    );

    assert_eq!(u32_from_u128(u32::MAX as u128), (0, 0, 0, u32::MAX));

    assert_eq!(u32_from_u128(u64::MAX as u128), (0, 0, u32::MAX, u32::MAX));

    assert_eq!(
        u32_from_u128((u64::MAX as u128) + u32::MAX as u128),
        (0, 1, 0, u32::MAX - 1)
    );

    assert_eq!(u32_from_u128(36_893_488_151_714_070_528), (0, 2, 1, 0));
}
