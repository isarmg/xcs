#[cfg(test)]
mod tests {
    // TODO: tests

    use super::*;

    #[test]
    fn max_to_max() {
        let (s, v) = read_generic_integer::<u32, u32>(u32::max_value()).unwrap();
        assert_eq!(s, Sign::Plus);
        assert_eq!(v, u32::max_value());
    }

    #[test]
    fn sign() {
        let (s, _) = read_generic_integer::<i8, i8>(0i8).unwrap();
        assert_eq!(s, Sign::Plus);

        let (s, _) = read_generic_integer::<i8, i8>(1i8).unwrap();
        assert_eq!(s, Sign::Plus);

        let (s, _) = read_generic_integer::<i8, i8>(-1i8).unwrap();
        assert_eq!(s, Sign::Minus);
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn generic_integer_biguint_has_expected_accessors() {
        assert_eq!(BigUint::_0(), BigUint::ZERO);
        assert_eq!(BigUint::_1(), BigUint::ONE);
        assert_eq!(BigUint::_10(), BigUint::from(10_u8));

        let zero_a = BigUint::_0r().expect("BigUint::_0r must be available");
        let zero_b = BigUint::_0r().expect("BigUint::_0r must be available");
        assert_eq!(*zero_a, BigUint::ZERO);
        assert!(std::ptr::eq(zero_a, zero_b));

        let one_a = BigUint::_1r().expect("BigUint::_1r must be available");
        let one_b = BigUint::_1r().expect("BigUint::_1r must be available");
        assert_eq!(*one_a, BigUint::ONE);
        assert!(std::ptr::eq(one_a, one_b));

        let ten_a = BigUint::_10r().expect("BigUint::_10r must be available");
        let ten_b = BigUint::_10r().expect("BigUint::_10r must be available");
        assert_eq!(*ten_a, BigUint::from(10_u8));
        assert!(std::ptr::eq(ten_a, ten_b));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn generic_integer_bigint_has_expected_accessors() {
        assert_eq!(BigInt::_0(), BigInt::ZERO);
        assert_eq!(BigInt::_1(), BigInt::ONE);
        assert_eq!(BigInt::_10(), BigInt::from(10_i8));

        assert!(BigInt::_10() > BigInt::ZERO);

        let zero_a = BigInt::_0r().expect("BigInt::_0r must be available");
        let zero_b = BigInt::_0r().expect("BigInt::_0r must be available");
        assert_eq!(*zero_a, BigInt::ZERO);
        assert!(std::ptr::eq(zero_a, zero_b));

        let one_a = BigInt::_1r().expect("BigInt::_1r must be available");
        let one_b = BigInt::_1r().expect("BigInt::_1r must be available");
        assert_eq!(*one_a, BigInt::ONE);
        assert!(std::ptr::eq(one_a, one_b));

        let ten_a = BigInt::_10r().expect("BigInt::_10r must be available");
        let ten_b = BigInt::_10r().expect("BigInt::_10r must be available");
        assert_eq!(*ten_a, BigInt::from(10_i8));
        assert!(std::ptr::eq(ten_a, ten_b));
    }
}
