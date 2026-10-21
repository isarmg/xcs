#[test]
fn test_sub_sign() {
    use crate::bigint::BigInt;
    use crate::traits::Num;

    fn sub_sign_i(a: &[BigDigit], b: &[BigDigit]) -> BigInt {
        let (sign, val) = sub_sign(a, b);
        BigInt::from_biguint(sign, val)
    }

    let a = BigUint::from_str_radix("265252859812191058636308480000000", 10).unwrap();
    let b = BigUint::from_str_radix("26525285981219105863630848000000", 10).unwrap();
    let a_i = BigInt::from(a.clone());
    let b_i = BigInt::from(b.clone());

    assert_eq!(sub_sign_i(&a.data, &b.data), &a_i - &b_i);
    assert_eq!(sub_sign_i(&b.data, &a.data), &b_i - &a_i);
}
