#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[cfg(feature = "arbitrary-precision")]
    fn parse_json(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    #[test_case(&json!({"minimum": 1_u64 << 54}), &json!((1_u64 << 54) - 1))]
    #[test_case(&json!({"minimum": 1_i64 << 54}), &json!((1_i64 << 54) - 1))]
    #[test_case(&json!({"maximum": 1_u64 << 54}), &json!((1_u64 << 54) + 1))]
    #[test_case(&json!({"maximum": 1_i64 << 54}), &json!((1_i64 << 54) + 1))]
    #[test_case(&json!({"exclusiveMinimum": 1_u64 << 54}), &json!(1_u64 << 54))]
    #[test_case(&json!({"exclusiveMinimum": 1_i64 << 54}), &json!(1_i64 << 54))]
    #[test_case(&json!({"exclusiveMinimum": 1_u64 << 54}), &json!((1_u64 << 54) - 1))]
    #[test_case(&json!({"exclusiveMinimum": 1_i64 << 54}), &json!((1_i64 << 54) - 1))]
    #[test_case(&json!({"exclusiveMaximum": 1_u64 << 54}), &json!(1_u64 << 54))]
    #[test_case(&json!({"exclusiveMaximum": 1_i64 << 54}), &json!(1_i64 << 54))]
    #[test_case(&json!({"exclusiveMaximum": 1_u64 << 54}), &json!((1_u64 << 54) + 1))]
    #[test_case(&json!({"exclusiveMaximum": 1_i64 << 54}), &json!((1_i64 << 54) + 1))]
    fn is_not_valid(schema: &Value, instance: &Value) {
        tests_util::is_not_valid(schema, instance);
    }

    #[test_case(&json!({"minimum": 5}), &json!(1), "/minimum")]
    #[test_case(&json!({"minimum": 6}), &json!(1), "/minimum")]
    #[test_case(&json!({"minimum": 7}), &json!(1), "/minimum")]
    #[test_case(&json!({"maximum": 5}), &json!(10), "/maximum")]
    #[test_case(&json!({"maximum": 6}), &json!(10), "/maximum")]
    #[test_case(&json!({"maximum": 7}), &json!(10), "/maximum")]
    #[test_case(&json!({"exclusiveMinimum": 5}), &json!(1), "/exclusiveMinimum")]
    #[test_case(&json!({"exclusiveMinimum": 6}), &json!(1), "/exclusiveMinimum")]
    #[test_case(&json!({"exclusiveMinimum": 7}), &json!(1), "/exclusiveMinimum")]
    #[test_case(&json!({"exclusiveMaximum": 5}), &json!(7), "/exclusiveMaximum")]
    #[test_case(&json!({"exclusiveMaximum": -1}), &json!(7), "/exclusiveMaximum")]
    #[test_case(&json!({"exclusiveMaximum": -1.0}), &json!(7), "/exclusiveMaximum")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    // Tests for arbitrary-precision feature - valid cases
    #[cfg(feature = "arbitrary-precision")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, r"18446744073709551617"; "minimum valid above u64_max")]
    #[test_case(r#"{"minimum": -9223372036854775809}"#, r"-9223372036854775808"; "minimum valid below i64_min")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, r"18446744073709551615"; "maximum valid below limit")]
    #[test_case(r#"{"exclusiveMinimum": 18446744073709551616}"#, r"18446744073709551617"; "exclusive_minimum valid")]
    #[test_case(r#"{"exclusiveMaximum": 18446744073709551616}"#, r"18446744073709551615"; "exclusive_maximum valid")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, "1e20"; "scientific notation above bigint minimum")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, "1e15"; "scientific notation below bigint maximum")]
    #[test_case(r#"{"minimum": 0}"#, "1e1000"; "minimum with huge positive number")]
    #[test_case(r#"{"minimum": -1e100}"#, "1e1000"; "minimum with huge positive above limit")]
    #[test_case(r#"{"exclusiveMinimum": 0}"#, "1e1000"; "exclusive minimum with huge positive")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, "1e20"; "bigint minimum with huge instance valid")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, "1e15"; "bigint maximum with huge instance valid")]
    #[test_case(r#"{"exclusiveMinimum": 18446744073709551616}"#, "1e25"; "bigint exclusive minimum with huge instance valid")]
    #[test_case(r#"{"minimum": 0}"#, "1e10000"; "infinity positive above minimum")]
    #[test_case(r#"{"maximum": 0}"#, "-1e10000"; "infinity negative below maximum")]
    #[test_case(r#"{"minimum": -100}"#, "1e10000"; "infinity positive above negative minimum")]
    #[test_case(r#"{"maximum": 100}"#, "-1e10000"; "infinity negative below positive maximum")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, "1e10000"; "infinity above bigint minimum")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, "-1e10000"; "infinity below bigint maximum")]
    #[test_case(r#"{"exclusiveMaximum": -9223372036854775808}"#, r"-9223372036854775809"; "exclusive_maximum valid just below i64_min limit")]
    #[test_case(r#"{"maximum": -9223372036854775808}"#, r"-9223372036854775809"; "maximum valid below i64_min limit")]
    #[test_case(r#"{"minimum": 18446744073709551615}"#, r"18446744073709551616"; "minimum valid just above u64_max limit")]
    #[test_case(r#"{"maximum": 1e2000000}"#, r"18446744073709551616"; "bigint instance below infinity maximum")]
    #[test_case(r#"{"minimum": -1e2000000}"#, r"-9223372036854775809"; "bigint instance above negative infinity minimum")]
    // Exponents below the f64 subnormal range underflow to a signed zero; the exact value still
    // sits strictly on one side of the limit.
    #[test_case(r#"{"exclusiveMinimum": 0}"#, "1e-400"; "underflowing positive above exclusive zero minimum")]
    #[test_case(r#"{"exclusiveMaximum": 0}"#, "-1e-400"; "underflowing negative below exclusive zero maximum")]
    #[test_case(r#"{"minimum": 0}"#, "1e-400"; "underflowing positive above zero minimum")]
    #[test_case(r#"{"maximum": 0}"#, "-1e-400"; "underflowing negative below zero maximum")]
    #[test_case(r#"{"maximum": 1}"#, "0.99999999999999999999999"; "instance f64 rounds onto an integer maximum")]
    fn is_valid_arbitrary_precision(schema_json: &str, instance_json: &str) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        tests_util::is_valid(&schema, &instance);
    }

    // Tests for arbitrary-precision feature - invalid cases
    #[cfg(feature = "arbitrary-precision")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, r"18446744073709551615"; "minimum invalid at u64_max")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, r"18446744073709551617"; "maximum invalid above limit")]
    #[test_case(r#"{"exclusiveMinimum": 18446744073709551616}"#, r"18446744073709551616"; "exclusive_minimum invalid at boundary")]
    #[test_case(r#"{"exclusiveMaximum": 18446744073709551616}"#, r"18446744073709551617"; "exclusive_maximum invalid above limit")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, "1e10"; "scientific notation below bigint minimum")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, "1e25"; "scientific notation above bigint maximum")]
    #[test_case(r#"{"maximum": 1e100}"#, "1e1000"; "maximum with huge number above limit")]
    #[test_case(r#"{"exclusiveMaximum": 1e100}"#, "1e1000"; "exclusive maximum with huge number")]
    #[test_case(r#"{"minimum": 1e100}"#, "-1e1000"; "minimum with huge negative below limit")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, "1e10"; "bigint minimum with huge instance invalid")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, "1e25"; "bigint maximum with huge instance invalid")]
    #[test_case(r#"{"exclusiveMaximum": 18446744073709551616}"#, "1e25"; "bigint exclusive maximum with huge instance invalid")]
    #[test_case(r#"{"maximum": 0}"#, "1e10000"; "infinity positive above maximum")]
    #[test_case(r#"{"minimum": 0}"#, "-1e10000"; "infinity negative below minimum")]
    #[test_case(r#"{"exclusiveMaximum": 100}"#, "1e10000"; "infinity positive above exclusive maximum")]
    #[test_case(r#"{"exclusiveMinimum": -100}"#, "-1e10000"; "infinity negative below exclusive minimum")]
    #[test_case(r#"{"maximum": 18446744073709551616}"#, "1e10000"; "infinity above bigint maximum")]
    #[test_case(r#"{"minimum": 18446744073709551616}"#, "-1e10000"; "infinity below bigint minimum")]
    #[test_case(r#"{"minimum": -9223372036854775808}"#, r"-9223372036854775809"; "minimum invalid just below i64_min limit")]
    #[test_case(r#"{"minimum": -9223372036854775808}"#, r"-9223372036854776807"; "minimum invalid within f64 rounding window below i64_min")]
    #[test_case(r#"{"exclusiveMinimum": -9223372036854775808}"#, r"-9223372036854775809"; "exclusive_minimum invalid below i64_min limit")]
    #[test_case(r#"{"maximum": 18446744073709551615}"#, r"18446744073709551616"; "maximum invalid just above u64_max limit")]
    #[test_case(r#"{"minimum": 1e2000000}"#, r"18446744073709551616"; "bigint instance below infinity minimum")]
    #[test_case(r#"{"maximum": -1e2000000}"#, r"-9223372036854775809"; "bigint instance above negative infinity maximum")]
    #[test_case(r#"{"maximum": -10000000000000000000000000.1}"#, r"-10000000000000000000000000"; "bigint instance above a fractional maximum f64 rounds onto it")]
    #[test_case(r#"{"minimum": 10000000000000000000000000.1}"#, r"10000000000000000000000000"; "bigint instance below a fractional minimum f64 rounds onto it")]
    // Exponents below the f64 subnormal range underflow to a signed zero; the exact value still
    // sits strictly on one side of the limit.
    #[test_case(r#"{"maximum": 0}"#, "1e-400"; "underflowing positive above zero maximum")]
    #[test_case(r#"{"minimum": 0}"#, "-1e-400"; "underflowing negative below zero minimum")]
    #[test_case(r#"{"exclusiveMaximum": 0}"#, "1e-400"; "underflowing positive above exclusive zero maximum")]
    #[test_case(r#"{"exclusiveMinimum": 0}"#, "-1e-400"; "underflowing negative below exclusive zero minimum")]
    #[test_case(r#"{"minimum": 1}"#, "0.99999999999999999999999"; "instance f64 rounds onto an integer minimum")]
    fn is_not_valid_arbitrary_precision(schema_json: &str, instance_json: &str) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        tests_util::is_not_valid(&schema, &instance);
    }

    // Regression test for BigInt limits exceeding i128 range with fractional instances
    #[cfg(feature = "arbitrary-precision")]
    #[test]
    fn bigint_limit_exceeds_i128_with_fraction() {
        // 10^50 is way beyond i128::MAX (~1.7e38)
        let limit = "1".to_string() + &"0".repeat(50);
        let schema_min = parse_json(&format!(r#"{{"minimum": {limit}}}"#));

        // Instance slightly below limit (10^50 - 1), should be INVALID
        let instance_below = "9".to_string() + &"9".repeat(49);
        let instance = parse_json(&instance_below);
        tests_util::is_not_valid(&schema_min, &instance);

        // Instance equal to limit, should be VALID
        let instance = parse_json(&limit);
        tests_util::is_valid(&schema_min, &instance);

        // Test maximum as well
        let schema_max = parse_json(&format!(r#"{{"maximum": {limit}}}"#));

        // Instance slightly above limit (10^50 + 1), should be INVALID
        let instance_above = "1".to_string() + &"0".repeat(50) + "1";
        let instance = parse_json(&instance_above);
        tests_util::is_not_valid(&schema_max, &instance);
    }

    // Test that schema compilation doesn't panic with extremely large scientific notation limits
    // These numbers (e.g., 1e10000) exceed f64 range and are treated as infinity for comparison
    #[cfg(feature = "arbitrary-precision")]
    #[test_case(r#"{"minimum": 1e10000}"#, "999999999999"; "huge positive scientific notation minimum")]
    #[test_case(r#"{"maximum": 1e10000}"#, "999999999999"; "huge positive scientific notation maximum")]
    #[test_case(r#"{"minimum": -1e10000}"#, "999999999999"; "huge negative scientific notation minimum")]
    #[test_case(r#"{"maximum": -1e10000}"#, "999999999999"; "huge negative scientific notation maximum")]
    #[test_case(r#"{"exclusiveMinimum": 1e10000}"#, "999999999999"; "huge positive scientific notation exclusive minimum")]
    #[test_case(r#"{"exclusiveMaximum": 1e10000}"#, "999999999999"; "huge positive scientific notation exclusive maximum")]
    #[test_case(r#"{"exclusiveMinimum": -1e10000}"#, "999999999999"; "huge negative scientific notation exclusive minimum")]
    #[test_case(r#"{"exclusiveMaximum": -1e10000}"#, "999999999999"; "huge negative scientific notation exclusive maximum")]
    fn extreme_scientific_notation_schema_limits_no_panic(schema_json: &str, instance_json: &str) {
        // This test ensures compilation doesn't panic when schema limits exceed f64 range
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        // The actual validation result doesn't matter - we're just ensuring no panic
        let compiled = crate::validator_for(&schema);
        assert!(compiled.is_ok(), "Schema compilation should not panic");

        // Also verify validation works
        let validator = compiled.unwrap();
        let _ = validator.is_valid(&instance);
    }

    // Test validation behavior with extremely large scientific notation schema limits
    #[cfg(feature = "arbitrary-precision")]
    #[test_case(r#"{"minimum": 1e10000}"#, "999999999999", false; "normal number vs huge positive minimum")]
    #[test_case(r#"{"maximum": 1e10000}"#, "999999999999", true; "normal number vs huge positive maximum")]
    #[test_case(r#"{"minimum": -1e10000}"#, "999999999999", true; "normal number vs huge negative minimum")]
    #[test_case(r#"{"maximum": -1e10000}"#, "999999999999", false; "normal number vs huge negative maximum")]
    fn extreme_scientific_notation_schema_limits_validation(
        schema_json: &str,
        instance_json: &str,
        expected_valid: bool,
    ) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected_valid {
            tests_util::is_valid(&schema, &instance);
        } else {
            tests_util::is_not_valid(&schema, &instance);
        }
    }

    #[test_case("minimum")]
    #[test_case("maximum")]
    #[test_case("exclusiveMinimum")]
    #[test_case("exclusiveMaximum")]
    fn malformed(keyword: &str) {
        tests_util::assert_compile_error(
            &json!({keyword: "a"}),
            "\"a\" is not of type \"number\"",
            &format!("/{keyword}"),
        );
    }
}
