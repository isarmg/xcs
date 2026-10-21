#[test]
fn test_pow_biguint() {
    let base = BigUint::from(5u8);
    let exponent = BigUint::from(3u8);

    assert_eq!(BigUint::from(125u8), base.pow(exponent));
}
