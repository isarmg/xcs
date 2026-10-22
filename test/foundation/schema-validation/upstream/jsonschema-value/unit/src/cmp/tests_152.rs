#[cfg(test)]
mod tests {
    use super::equal;
    use json_codec::{json, Value};
    use test_case::test_case;

    #[test_case(&json!(1), &json!(1.0))]
    #[test_case(&json!([2]), &json!([2.0]))]
    #[test_case(&json!([-3]), &json!([-3.0]))]
    #[test_case(&json!({"a": 1}), &json!({"a": 1.0}))]
    fn are_equal(left: &Value, right: &Value) {
        assert!(equal(left, right));
    }

    #[test_case(&json!(1), &json!(2.0))]
    #[test_case(&json!([]), &json!(["foo"]))]
    #[test_case(&json!([-3]), &json!([-4.0]))]
    #[test_case(&json!({"a": 1}), &json!({"a": 1.0, "b": 2}))]
    fn are_not_equal(left: &Value, right: &Value) {
        assert!(!equal(left, right));
    }

    #[cfg(feature = "arbitrary-precision")]
    mod arbitrary_precision {
        use super::equal;
        use json_codec::Value;
        use test_case::test_case;

        fn parse_json(s: &str) -> Value {
            json_codec::from_str(s).unwrap()
        }
        #[test_case("0.1", "0.1", true; "exact decimal match")]
        #[test_case("0.1", "0.10", true; "decimal with trailing zero")]
        #[test_case("0.1", "0.100000", true; "decimal with many trailing zeros")]
        #[test_case("0.1", "0.2", false; "different decimals")]
        #[test_case("0.3", "0.30", true; "another trailing zero case")]
        #[test_case("1.0", "1", true; "decimal vs integer")]
        #[test_case("1.00", "1.0", true; "decimals with different trailing zeros")]
        #[test_case(
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.5",
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.5",
            true;
            "huge decimal self equality"
        )]
        #[test_case(
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.5",
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.6",
            false;
            "huge decimals different"
        )]
        #[test_case(
            "99999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999.5",
            "100",
            false;
            "huge decimal vs small integer"
        )]
        #[test_case("18446744073709551616", "18446744073709551616", true; "large integer self equality")]
        #[test_case("18446744073709551616", "18446744073709551617", false; "large integers different")]
        #[test_case("99999999999999999999999999999999999999", "99999999999999999999999999999999999999", true; "very large integer equality")]
        #[test_case("99999999999999999999999999999999999999", "100", false; "very large vs small integer")]
        #[test_case("100", "100.0", true; "small integer vs decimal")]
        #[test_case("0.1", "1", false; "small decimal vs integer")]
        #[test_case("1.0", "1", true; "decimal one vs integer one")]
        #[test_case("18446744073709551616", "100.5", false; "large int vs decimal")]
        #[test_case("18446744073709551616.0", "18446744073709551616", true; "large int as decimal vs large int")]
        #[test_case("-0.1", "-0.1", true; "negative decimal equality")]
        #[test_case("-0.1", "-0.10", true; "negative decimal trailing zero")]
        #[test_case("-18446744073709551616", "-18446744073709551616", true; "negative large int")]
        #[test_case("-18446744073709551616", "18446744073709551616", false; "negative vs positive large")]
        #[test_case("-100.5", "-100.5", true; "negative decimal match")]
        #[test_case("-100.5", "100.5", false; "negative vs positive decimal")]
        #[test_case("0", "0.0", true; "zero integer vs decimal")]
        #[test_case("0.0", "0.00", true; "zero decimals with different precision")]
        #[test_case("-0.0", "0.0", true; "negative zero vs positive zero")]
        #[test_case("1e10", "10000000000", true; "scientific notation vs integer")]
        #[test_case("1e19", "10000000000000000000", true; "scientific integer beyond i64")]
        #[test_case("1e19", "10000000000000000001", false; "scientific integer mismatch")]
        #[test_case("1.5e2", "150", true; "decimal scientific vs integer")]
        #[test_case("1.5e2", "150.0", true; "decimal scientific vs decimal")]
        #[test_case(r"[0.1, 0.2, 0.3]", r"[0.1, 0.2, 0.3]", true; "array exact match")]
        #[test_case(r"[0.1, 0.2]", r"[0.10, 0.20]", true; "array with trailing zeros")]
        #[test_case(r"[18446744073709551616]", r"[18446744073709551616]", true; "array with large integer")]
        #[test_case(r"[0.1, 0.2]", r"[0.1, 0.3]", false; "array different values")]
        #[test_case(r#"{"value": 0.1}"#, r#"{"value": 0.1}"#, true; "object exact match")]
        #[test_case(r#"{"value": 0.1}"#, r#"{"value": 0.10}"#, true; "object with trailing zero")]
        #[test_case(r#"{"id": 18446744073709551616}"#, r#"{"id": 18446744073709551616}"#, true; "object with large integer")]
        #[test_case(r#"{"value": 0.1}"#, r#"{"value": 0.2}"#, false; "object different values")]
        #[test_case("18446744073709551616", "-1", false; "large positive bigint vs negative i64")]
        #[test_case("18446744073709551616", "-100", false; "large positive bigint vs negative i64 small")]
        #[test_case("-18446744073709551616", "-1", false; "large negative bigint vs small negative i64")]
        #[test_case("18446744073709551616", "1e10", false; "large bigint vs scientific notation f64")]
        #[test_case("10000000000", "1e10", true; "bigint vs scientific notation equal")]
        #[test_case("-18446744073709551616", "-1.5e3", false; "negative bigint vs scientific notation")]
        #[test_case("0.5", "5e-1", true; "bigfraction vs scientific notation equal")]
        #[test_case("0.3", "3e-1", true; "bigfraction vs scientific equal exact")]
        #[test_case("123.456", "1.23456e2", true; "bigfraction vs scientific notation")]
        #[test_case("0.1", "1e-2", false; "bigfraction vs scientific not equal")]
        #[test_case("1e309", "1e309", true; "huge scientific notation now handled")]
        #[test_case("1e400", "1e400", true; "extreme scientific notation now handled")]
        #[test_case("1e-400", "1e-400", true; "extreme small scientific notation self equality")]
        #[test_case("1e309", "1", false; "huge scientific notation vs integer")]
        fn arbitrary_precision_equality(left_str: &str, right_str: &str, should_equal: bool) {
            let left = parse_json(left_str);
            let right = parse_json(right_str);
            assert_eq!(equal(&left, &right), should_equal);
        }
    }
}
