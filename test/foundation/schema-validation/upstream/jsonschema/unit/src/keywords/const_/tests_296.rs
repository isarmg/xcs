#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"const": 1}), &json!(2), "/const")]
    #[test_case(&json!({"const": null}), &json!(3), "/const")]
    #[test_case(&json!({"const": false}), &json!(4), "/const")]
    #[test_case(&json!({"const": []}), &json!(5), "/const")]
    #[test_case(&json!({"const": {}}), &json!(6), "/const")]
    #[test_case(&json!({"const": ""}), &json!(7), "/const")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    // Tests for arbitrary-precision const validation
    #[cfg(feature = "arbitrary-precision")]
    mod arbitrary_precision {
        use crate::tests_util;
        use json_codec::Value;
        use test_case::test_case;

        fn parse_json(json: &str) -> Value {
            json_codec::from_str(json).unwrap()
        }

        #[test_case(r#"{"const": 18446744073709551617}"#, "18446744073709551617", true; "large int exact match")]
        #[test_case(r#"{"const": 18446744073709551617}"#, "18446744073709551616", false; "large int different by one")]
        #[test_case(r#"{"const": 18446744073709551617}"#, "18446744073709551618", false; "large int different by one above")]
        #[test_case(r#"{"const": -9223372036854775809}"#, "-9223372036854775809", true; "large negative int match")]
        #[test_case(r#"{"const": -9223372036854775809}"#, "-9223372036854775808", false; "large negative int different")]
        #[test_case(r#"{"const": 0.1}"#, "0.1", true; "decimal exact match")]
        #[test_case(r#"{"const": 0.1}"#, "0.10000000000000001", false; "decimal precision difference")]
        fn const_arbitrary_precision(schema_json: &str, instance_json: &str, expected_valid: bool) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected_valid {
                tests_util::is_valid(&schema, &instance);
            } else {
                tests_util::is_not_valid(&schema, &instance);
            }
        }
    }
}
