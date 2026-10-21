#[test]
fn test_u128_u32_roundtrip() {
    // roundtrips
    let values = vec![
        0u128,
        1u128,
        u64::MAX as u128 * 3,
        u32::MAX as u128,
        u64::MAX as u128,
        (u64::MAX as u128) + u32::MAX as u128,
        u128::MAX,
    ];

    for val in &values {
        let (a, b, c, d) = u32_from_u128(*val);
        assert_eq!(u32_to_u128(a, b, c, d), *val);
    }
}
