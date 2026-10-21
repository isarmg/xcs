#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"type": "array"}), &json!(1), "/type")]
    #[test_case(&json!({"type": "boolean"}), &json!(1), "/type")]
    #[test_case(&json!({"type": "integer"}), &json!("f"), "/type")]
    #[test_case(&json!({"type": "null"}), &json!(1), "/type")]
    #[test_case(&json!({"type": "number"}), &json!("f"), "/type")]
    #[test_case(&json!({"type": "object"}), &json!(1), "/type")]
    #[test_case(&json!({"type": "string"}), &json!(1), "/type")]
    #[test_case(&json!({"type": ["string", "object"]}), &json!(1), "/type")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    #[test_case(&json!({"type": 5}), "5 is not of types \"string\", \"array\""; "not string or array")]
    #[test_case(&json!({"type": "foo"}), "Unexpected type"; "unknown name")]
    #[test_case(&json!({"type": ["foo"]}), "Unexpected type"; "single unknown name")]
    #[test_case(&json!({"type": [5]}), "5 is not of type \"string\""; "single non string")]
    #[test_case(&json!({"type": ["string", "foo"]}), "\"foo\" is not one of \"array\", \"boolean\" or 5 other candidates"; "list unknown name")]
    #[test_case(&json!({"type": [5, "string"]}), "5 is not of type \"string\""; "list non string")]
    fn malformed(schema: &Value, message: &str) {
        tests_util::assert_compile_error(schema, message, "/type");
    }

    fn parse_json(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test_case(r#"{"type": "integer"}"#, "42", true; "plain integer")]
    #[test_case(r#"{"type": "integer"}"#, "-42", true; "negative integer")]
    #[test_case(r#"{"type": "integer"}"#, "0", true; "zero")]
    #[test_case(r#"{"type": "integer"}"#, "1.0", true; "float with .0")]
    #[test_case(r#"{"type": "integer"}"#, "42.0", true; "integer as float")]
    #[test_case(r#"{"type": "integer"}"#, "-42.0", true; "negative float with .0")]
    #[test_case(r#"{"type": "integer"}"#, "1.5", false; "decimal")]
    #[test_case(r#"{"type": "integer"}"#, "0.1", false; "small decimal")]
    #[test_case(r#"{"type": "integer"}"#, "42.7", false; "float with decimal")]
    #[test_case(r#"{"type": "integer"}"#, "9223372036854775807", true; "i64::MAX")]
    #[test_case(r#"{"type": "integer"}"#, "-9223372036854775808", true; "i64::MIN")]
    #[test_case(r#"{"type": "integer"}"#, "18446744073709551615", true; "u64::MAX")]
    #[test_case(r#"{"type": "integer"}"#, "1e10", true; "scientific int 1e10")]
    #[test_case(r#"{"type": "integer"}"#, "1.0e10", true; "scientific 1.0e10")]
    #[test_case(r#"{"type": "integer"}"#, "1.5e10", true; "scientific 1.5e10 equals 15000000000")]
    #[test_case(r#"{"type": "integer"}"#, "1e-10", false; "scientific small 1e-10")]
    #[test_case(r#"{"type": "integer"}"#, "true", false; "boolean")]
    #[test_case(r#"{"type": "integer"}"#, r#""42""#, false; "string")]
    #[test_case(r#"{"type": "integer"}"#, "[]", false; "array")]
    #[test_case(r#"{"type": "integer"}"#, "{}", false; "object")]
    #[test_case(r#"{"type": "integer"}"#, "null", false; "null")]
    fn integer_type_validation(schema_json: &str, instance_json: &str, expected: bool) {
        let schema = parse_json(schema_json);
        let instance = parse_json(instance_json);
        if expected {
            tests_util::is_valid(&schema, &instance);
        } else {
            tests_util::is_not_valid(&schema, &instance);
        }
    }

    #[cfg(feature = "arbitrary-precision")]
    mod arbitrary_precision {
        use crate::tests_util;
        use serde_json::Value;
        use test_case::test_case;

        fn parse_json(s: &str) -> Value {
            serde_json::from_str(s).unwrap()
        }

        #[test_case(r#"{"type": "integer"}"#, "18446744073709551616", true; "u64_max_plus_1_plain")]
        #[test_case(r#"{"type": "integer"}"#, "18446744073709551616.0", true; "u64_max_plus_1_with_dot_0")]
        #[test_case(r#"{"type": "integer"}"#, "99999999999999999999", true; "huge_plain_integer")]
        #[test_case(r#"{"type": "integer"}"#, "99999999999999999999.0", true; "huge_integer_with_dot_0")]
        #[test_case(
            r#"{"type": "integer"}"#,
            "999999999999999999999999999999999999999999999999999999999999999999999999999999",
            true;
            "very_huge_plain_integer"
        )]
        #[test_case(
            r#"{"type": "integer"}"#,
            "999999999999999999999999999999999999999999999999999999999999999999999999999999.0",
            true;
            "very_huge_integer_with_dot_0"
        )]
        #[test_case(r#"{"type": "integer"}"#, "-18446744073709551616", true; "negative_huge_integer")]
        #[test_case(r#"{"type": "integer"}"#, "-18446744073709551616.0", true; "negative_huge_integer_with_dot_0")]
        #[test_case(r#"{"type": "integer"}"#, "-99999999999999999999", true; "negative_huge_plain")]
        #[test_case(r#"{"type": "integer"}"#, "-99999999999999999999.0", true; "negative_huge_plain_with_dot_0")]
        #[test_case(r#"{"type": "integer"}"#, "18446744073709551616.5", false; "huge decimal")]
        #[test_case(r#"{"type": "integer"}"#, "99999999999999999999.5", false; "huge float")]
        #[test_case(
            r#"{"type": "integer"}"#,
            "999999999999999999999999999999999999999999999999999999999999999999999999999999.5",
            false;
            "very huge float"
        )]
        #[test_case(r#"{"type": "integer"}"#, "0.3", false; "bigfrac 0.3")]
        #[test_case(r#"{"type": "integer"}"#, "0.1", false; "bigfrac 0.1")]
        #[test_case(r#"{"type": "integer"}"#, "123.456", false; "bigfrac 123.456")]
        #[test_case(r#"{"type": "integer"}"#, "1e1000", true; "huge scientific notation integer")]
        #[test_case(r#"{"type": "integer"}"#, "1e1000001", false; "infinity positive")]
        #[test_case(r#"{"type": "integer"}"#, "-1e1000001", false; "infinity negative")]
        #[test_case(r#"{"type": "integer"}"#, "1.5e-1000", false; "huge scientific float")]
        #[test_case(r#"{"type": "number"}"#, "18446744073709551616", true; "huge int valid as number")]
        #[test_case(r#"{"type": "number"}"#, "18446744073709551616.0", true; "huge .0 valid as number")]
        #[test_case(r#"{"type": "number"}"#, "18446744073709551616.5", true; "huge float valid as number")]
        #[test_case(r#"{"type": "number"}"#, "1e10000", true; "infinity valid as number")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "18446744073709551616", true; "huge int in union")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "18446744073709551616.0", true; "huge .0 in union")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, "18446744073709551616.5", false; "huge float not in union")]
        #[test_case(r#"{"type": ["integer", "string"]}"#, r#""foo""#, true; "string in union")]
        #[test_case(r#"{"type": ["number", "string"]}"#, "18446744073709551616.5", true; "huge float with number")]
        #[test_case(r#"{"type": "integer"}"#, "true", false; "boolean not integer")]
        #[test_case(r#"{"type": "integer"}"#, r#""42""#, false; "string not integer")]
        #[test_case(r#"{"type": "integer"}"#, "[]", false; "array not integer")]
        #[test_case(r#"{"type": "integer"}"#, "{}", false; "object not integer")]
        #[test_case(r#"{"type": "integer"}"#, "null", false; "null not integer")]
        fn huge_number_integer_validation(schema_json: &str, instance_json: &str, expected: bool) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected {
                tests_util::is_valid(&schema, &instance);
            } else {
                tests_util::is_not_valid(&schema, &instance);
            }
        }
    }
}
