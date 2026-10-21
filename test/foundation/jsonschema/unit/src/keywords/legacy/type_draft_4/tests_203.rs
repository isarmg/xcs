#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::Value;
    use test_case::test_case;

    fn parse_json(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    // Draft 4 is strict: floats like 1.0 are NOT integers
    #[test_case(r#"{"type": "integer"}"#, "42", true; "plain integer")]
    #[test_case(r#"{"type": "integer"}"#, "-42", true; "negative integer")]
    #[test_case(r#"{"type": "integer"}"#, "0", true; "zero")]
    #[test_case(r#"{"type": "integer"}"#, "1.0", false; "float with .0 is not integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "42.0", false; "integer as float is not integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "-42.0", false; "negative float with .0 is not integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "1.5", false; "decimal")]
    #[test_case(r#"{"type": "integer"}"#, "0.1", false; "small decimal")]
    #[test_case(r#"{"type": "integer"}"#, "42.7", false; "float with decimal")]
    #[test_case(r#"{"type": "integer"}"#, "9223372036854775807", true; "i64::MAX")]
    #[test_case(r#"{"type": "integer"}"#, "-9223372036854775808", true; "i64::MIN")]
    #[test_case(r#"{"type": "integer"}"#, "18446744073709551615", true; "u64::MAX")]
    #[test_case(r#"{"type": "integer"}"#, "true", false; "boolean")]
    #[test_case(r#"{"type": "integer"}"#, r#""42""#, false; "string")]
    #[test_case(r#"{"type": "integer"}"#, "[]", false; "array")]
    #[test_case(r#"{"type": "integer"}"#, "{}", false; "object")]
    #[test_case(r#"{"type": "integer"}"#, "null", false; "null")]
    #[test_case(r#"{"type": "integer"}"#, "1e2", false; "exponent notation is not an integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "1E2", false; "capital exponent notation is not an integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "1e0", false; "zero exponent is not an integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "-1e2", false; "negative exponent notation is not an integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "1e16", false; "exponent past the f64 fixed-notation cutoff is not an integer in draft4")]
    #[test_case(r#"{"type": "integer"}"#, "1.0e2", false; "fraction and exponent together is not an integer in draft4")]
    fn integer_type_validation_draft4(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid_with_draft4(&schema, &instance);
        } else {
            tests_util::is_not_valid_with_draft4(&schema, &instance);
        }
    }

    #[test_case(r#"{"type": 5}"#, "5 is not of types \"string\", \"array\""; "not string or array")]
    #[test_case(r#"{"type": "foo"}"#, "Unexpected type"; "unknown name")]
    #[test_case(r#"{"type": ["foo"]}"#, "Unexpected type"; "single unknown name")]
    #[test_case(r#"{"type": [5]}"#, "5 is not of type \"string\""; "single non string")]
    #[test_case(r#"{"type": ["string", "foo"]}"#, "\"foo\" is not one of \"array\", \"boolean\" or 5 other candidates"; "list unknown name")]
    #[test_case(r#"{"type": [5, "string"]}"#, "5 is not of type \"string\""; "list non string")]
    fn malformed_draft4(schema_json: &str, message: &str) {
        tests_util::assert_compile_error_with(
            &crate::options().with_draft(crate::Draft::Draft4),
            &parse_json(schema_json),
            message,
            "/type",
        );
    }

    #[cfg(feature = "arbitrary-precision")]
    mod arbitrary_precision {
        use crate::tests_util;
        use serde_json::Value;
        use test_case::test_case;

        fn parse_json(s: &str) -> Value {
            serde_json::from_str(s).unwrap()
        }

        // Tests for huge integers beyond i64/u64 range - these must be parsed from JSON string
        // to avoid Python/Rust int conversion issues
        #[test_case(r#"{"type": "integer"}"#, "18446744073709551616", true; "u64_max_plus_1_plain")]
        #[test_case(r#"{"type": "integer"}"#, "-9223372036854775809", true; "i64_min_minus_1")]
        #[test_case(r#"{"type": "integer"}"#, "99999999999999999999", true; "huge_plain_integer")]
        #[test_case(
            r#"{"type": "integer"}"#,
            "999999999999999999999999999999999999999999999999999999999999999999999999999999",
            true;
            "very_huge_plain_integer"
        )]
        #[test_case(r#"{"type": "integer"}"#, "-18446744073709551616", true; "negative_huge_integer")]
        #[test_case(r#"{"type": "integer"}"#, "-99999999999999999999", true; "negative_huge_plain")]
        // Numbers with decimal points are NOT integers in Draft 4, even if fractional part is zero
        #[test_case(r#"{"type": "integer"}"#, "18446744073709551616.0", false; "huge_with_dot_0_not_integer")]
        #[test_case(r#"{"type": "integer"}"#, "99999999999999999999.0", false; "huge_integer_with_dot_0_not_integer")]
        #[test_case(r#"{"type": "integer"}"#, "-18446744073709551616.0", false; "negative_huge_with_dot_0_not_integer")]
        #[test_case(r#"{"type": "integer"}"#, "-99999999999999999999.0", false; "negative_very_huge_with_dot_0_not_integer")]
        #[test_case(r#"{"type": "integer"}"#, "18446744073709551616.5", false; "huge decimal")]
        #[test_case(r#"{"type": "integer"}"#, "99999999999999999999.5", false; "huge float")]
        #[test_case(
            r#"{"type": "integer"}"#,
            "999999999999999999999999999999999999999999999999999999999999999999999999999999.5",
            false;
            "very huge float"
        )]
        // Only decidable while the literal survives; otherwise these come back in exponent form.
        #[test_case(r#"{"type": "integer"}"#, "1e300", false; "large exponent is not an integer in draft4")]
        #[test_case(r#"{"type": "integer"}"#, "100000000000000000000.0", false; "large float with a fraction part is not an integer in draft4")]
        #[test_case(r#"{"type": "integer"}"#, "1e1000", false; "huge scientific notation is not an integer in draft4")]
        #[test_case(r#"{"type": "integer"}"#, "1e1000001", false; "scientific notation past f64 is not an integer in draft4")]
        #[test_case(r#"{"type": "integer"}"#, "-1e1000001", false; "negative scientific notation past f64 is not an integer in draft4")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "18446744073709551616", true; "huge int in union")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "-9223372036854775809", true; "huge negative int in union")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "18446744073709551616.0", false; "huge .0 not integer in union")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "18446744073709551616.5", false; "huge float not in union")]
        fn huge_number_integer_validation_draft4(
            schema_json: &str,
            instance_json: &str,
            expected: bool,
        ) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected {
                tests_util::is_valid_with_draft4(&schema, &instance);
            } else {
                tests_util::is_not_valid_with_draft4(&schema, &instance);
            }
        }
    }
}
