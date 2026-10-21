#[cfg(all(test, feature = "arbitrary-precision"))]
mod bignum_tests {
    use crate::numeric::bignum;
    use fraction::BigFraction;
    use ::numeric::bigint::BigInt;
    use json_codec::{Number, Value};
    use std::cmp::Ordering;
    use test_case::test_case;

    fn number_from_str(raw: &str) -> Number {
        match json_codec::from_str::<Value>(raw).expect("valid JSON number") {
            Value::Number(num) => num,
            _ => unreachable!(),
        }
    }

    #[test_case("18446744073709551616", u64::MAX, Ordering::Greater; "above u64 limit")]
    fn compare_bigint_to_u64_limit(big: &str, limit: u64, expected: Ordering) {
        let big = BigInt::parse_bytes(big.as_bytes(), 10).unwrap();
        assert_eq!(bignum::compare_bigint_to_limit(&big, limit), Some(expected));
    }

    #[test_case("-18446744073709551616", i64::MIN, Ordering::Less; "below i64 limit")]
    fn compare_bigint_to_i64_limit(big: &str, limit: i64, expected: Ordering) {
        let big = BigInt::parse_bytes(big.as_bytes(), 10).unwrap();
        assert_eq!(bignum::compare_bigint_to_limit(&big, limit), Some(expected));
    }

    // Infinity limits come from schema numbers beyond the exponent cap (e.g. `1e2000000`);
    // limits without an exact integer form defer to the caller's f64 comparison.
    #[test_case(f64::INFINITY, Some(Ordering::Less); "infinity limit")]
    #[test_case(f64::NEG_INFINITY, Some(Ordering::Greater); "negative infinity limit")]
    #[test_case(0.5, None; "no exact integer form")]
    fn compare_bigint_to_f64_limit(limit: f64, expected: Option<Ordering>) {
        let big = BigInt::parse_bytes(b"18446744073709551616", 10).unwrap();
        assert_eq!(bignum::compare_bigint_to_limit(&big, limit), expected);
    }

    #[test]
    fn bigint_parses_scientific_integer() {
        let num = number_from_str("1e19");
        let parsed = bignum::try_parse_bigint(&num).expect("parsed bigint");
        assert_eq!(
            parsed,
            BigInt::parse_bytes(b"10000000000000000000", 10).unwrap()
        );
    }

    #[test]
    fn bigint_rejects_non_integer_scientific() {
        let num = number_from_str("1.25e1");
        assert!(bignum::try_parse_bigint(&num).is_none());
    }

    #[test]
    fn bigfraction_parses_scientific_decimal() {
        let num = number_from_str("1.5e-5");
        let parsed = bignum::try_parse_bigfraction(&num).expect("parsed bigfraction");
        let expected =
            BigFraction::from(BigInt::from(3)) / BigFraction::from(BigInt::from(200_000));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn bigfraction_skips_scientific_integer() {
        let num = number_from_str("3e4");
        assert!(bignum::try_parse_bigfraction(&num).is_none());
    }
}
