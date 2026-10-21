#[cfg(test)]
mod tests {
    use json_codec::Number;
    #[cfg(feature = "validator")]
    use json_codec::Value;
    use jsonschema_value::{cmp, numeric};

    fn number(text: &str) -> Number { json_codec::from_str(text).unwrap() }

    #[test]
    fn bigint_precision_crosses_u64_and_f64_boundaries() {
        let lower = number("18446744073709551616");
        let upper = number("18446744073709551617");
        assert_ne!(lower.to_string(), upper.to_string());
        assert!(!cmp::equal_numbers(&lower, &upper));
        let big = numeric::bignum::try_parse_bigint(&lower).unwrap();
        assert_eq!(big.to_string(), "18446744073709551616");
        assert!(numeric::gt(&lower, u64::MAX));
        assert!(!numeric::le(&lower, u64::MAX));
        assert!(numeric::is_multiple_of_integer(&lower, 2.0));
        assert!(!numeric::is_multiple_of_integer(&upper, 2.0));
    }

    #[test]
    fn exact_rational_keeps_fraction_beyond_f64_mantissa() {
        let ratio = numeric::bignum::try_parse_bigfraction(&number("9007199254740993.5")).unwrap();
        assert_eq!(ratio.numer().unwrap().to_string(), "18014398509481987");
        assert_eq!(ratio.denom().unwrap().to_string(), "2");
        let small = numeric::bignum::try_parse_bigfraction(&number("1.5e-5")).unwrap();
        assert_eq!(small.numer().unwrap().to_string(), "3");
        assert_eq!(small.denom().unwrap().to_string(), "200000");
        assert!(numeric::is_multiple_of_float(&number("0.000045"), 0.000015));
        assert!(!numeric::is_multiple_of_float(&number("0.000046"), 0.000015));
    }

    #[test]
    fn exponents_remain_exact_outside_f64_range() {
        let huge = numeric::bignum::try_parse_bigint(&number("1e400")).unwrap();
        assert_eq!(huge.to_string(), format!("1{}", "0".repeat(400)));
        let tiny = number("1e-400");
        assert!(!cmp::equal_numbers(&tiny, &number("0")));
        assert!(!numeric::is_multiple_of_float(&tiny, 0.5));
        assert!(numeric::is_multiple_of_integer(&number("1e400"), 4.0));
        assert!(!numeric::is_multiple_of_integer(&number("1e400"), 3.0));
    }

    #[cfg(feature = "validator")]
    fn validate(schema: &str, accepted: &[&str], rejected: &[&str]) {
        let schema: Value = json_codec::from_str(schema).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for literal in accepted {
            let instance: Value = json_codec::from_str(literal).unwrap();
            assert!(validator.is_valid(&instance), "must accept {literal} for {schema}");
            assert!(validator.validate(&instance).is_ok());
        }
        for literal in rejected {
            let instance: Value = json_codec::from_str(literal).unwrap();
            assert!(!validator.is_valid(&instance), "must reject {literal} for {schema}");
            assert!(validator.validate(&instance).is_err());
        }
    }

    #[cfg(feature = "validator")]
    #[test]
    fn validator_preserves_adjacent_large_integer_bounds() {
        validate(r#"{"type":"integer","minimum":18446744073709551616,"maximum":18446744073709551618}"#,
            &["18446744073709551616", "18446744073709551617", "18446744073709551618"],
            &["18446744073709551615", "18446744073709551619", "18446744073709551616.5"]);
        validate(r#"{"maximum":-18446744073709551616}"#,
            &["-18446744073709551616", "-18446744073709551617"], &["-18446744073709551615"]);
    }

    #[cfg(feature = "validator")]
    #[test]
    fn validator_multiple_of_uses_large_integer_and_exact_rational_paths() {
        validate(r#"{"multipleOf":1000000000000000000000000000003}"#,
            &["1000000000000000000000000000003", "2000000000000000000000000000006"],
            &["1000000000000000000000000000004", "2000000000000000000000000000007"]);
        validate(r#"{"multipleOf":0.000015}"#,
            &["0.000045", "0.000030", "-0.000045"], &["0.000046", "0.000031"]);
    }

    #[cfg(feature = "validator")]
    #[test]
    fn validator_tiny_decimal_bounds_do_not_underflow_to_zero() {
        validate(r#"{"minimum":1e-400,"maximum":3e-400}"#,
            &["1e-400", "2e-400", "3e-400"], &["0", "-1e-400", "4e-400"]);
    }
}
