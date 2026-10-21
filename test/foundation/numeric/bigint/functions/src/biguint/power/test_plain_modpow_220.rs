#[test]
fn test_plain_modpow() {
    let two = &BigUint::from(2u32);
    let modulus = BigUint::from(0x1100u32);

    let exp = vec![0, 0b1];
    assert_eq!(
        two.pow(0b1_00000000_u32) % &modulus,
        plain_modpow(two, &exp, &modulus)
    );
    let exp = vec![0, 0b10];
    assert_eq!(
        two.pow(0b10_00000000_u32) % &modulus,
        plain_modpow(two, &exp, &modulus)
    );
    let exp = vec![0, 0b110010];
    assert_eq!(
        two.pow(0b110010_00000000_u32) % &modulus,
        plain_modpow(two, &exp, &modulus)
    );
    let exp = vec![0b1, 0b1];
    assert_eq!(
        two.pow(0b1_00000001_u32) % &modulus,
        plain_modpow(two, &exp, &modulus)
    );
    let exp = vec![0b1100, 0, 0b1];
    assert_eq!(
        two.pow(0b1_00000000_00001100_u32) % &modulus,
        plain_modpow(two, &exp, &modulus)
    );
}
