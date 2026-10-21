#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"enum": [1]}), &json!(2), "/enum")]
    #[test_case(&json!({"enum": [1, 3]}), &json!(2), "/enum")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    // 10 entries — exercises BigStringEnumValidator
    const BIG_STRING_ENUM: &str = r#"{
        "enum": ["a","b","c","d","e","f","g","h","i","j","k"]
    }"#;

    #[test]
    fn big_string_enum_valid() {
        let schema: Value = json_codec::from_str(BIG_STRING_ENUM).unwrap();
        for s in &["a", "e", "j"] {
            tests_util::is_valid(&schema, &json!(s));
        }
    }

    #[test]
    fn big_string_enum_invalid_string() {
        let schema: Value = json_codec::from_str(BIG_STRING_ENUM).unwrap();
        tests_util::is_not_valid(&schema, &json!("z"));
    }

    #[test]
    fn big_string_enum_invalid_type() {
        let schema: Value = json_codec::from_str(BIG_STRING_ENUM).unwrap();
        tests_util::is_not_valid(&schema, &json!(1));
        tests_util::is_not_valid(&schema, &json!(null));
    }

    #[test]
    fn big_string_enum_location() {
        let schema: Value = json_codec::from_str(BIG_STRING_ENUM).unwrap();
        tests_util::assert_schema_location(&schema, &json!("z"), "/enum");
    }

    // 11 strings plus null — the big string set with `null` allowed
    const BIG_STRING_OR_NULL_ENUM: &str = r#"{
        "enum": ["a","b","c","d","e","f","g","h","i","j","k",null]
    }"#;

    #[test_case(&json!({"enum": ["a", "b", null]}), &json!(null); "small accepts null")]
    #[test_case(&json!({"enum": ["a", "b", null]}), &json!("b"); "small accepts string")]
    #[test_case(&json!({"enum": [null, "a"]}), &json!("a"); "null first accepts string")]
    #[test_case(&json_codec::from_str(BIG_STRING_OR_NULL_ENUM).unwrap(), &json!(null); "big accepts null")]
    #[test_case(&json_codec::from_str(BIG_STRING_OR_NULL_ENUM).unwrap(), &json!("k"); "big accepts string")]
    fn string_or_null_enum_valid(schema: &Value, instance: &Value) {
        tests_util::is_valid(schema, instance);
    }

    #[test_case(&json!({"enum": ["a", "b", null]}), &json!("z"); "small rejects other string")]
    #[test_case(&json!({"enum": ["a", "b", null]}), &json!(1); "small rejects number")]
    #[test_case(&json!({"enum": ["a", "b"]}), &json!(null); "small without null rejects null")]
    #[test_case(&json_codec::from_str(BIG_STRING_OR_NULL_ENUM).unwrap(), &json!("z"); "big rejects other string")]
    #[test_case(&json_codec::from_str(BIG_STRING_OR_NULL_ENUM).unwrap(), &json!(false); "big rejects boolean")]
    #[test_case(&json_codec::from_str(BIG_STRING_ENUM).unwrap(), &json!(null); "big without null rejects null")]
    fn string_or_null_enum_invalid(schema: &Value, instance: &Value) {
        tests_util::is_not_valid(schema, instance);
    }

    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(2); "integer accepts member")]
    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(2.0); "integer accepts integral float")]
    #[test_case(&json!({"enum": [-1, 0]}), &json!(-0.0); "integer accepts negative zero")]
    #[test_case(&json!({"enum": [null, 10, 20]}), &json!(null); "integer with null accepts null")]
    #[test_case(&json!({"enum": [null, 10, 20]}), &json!(10); "null first accepts integer")]
    #[test_case(&json!({"enum": [9_007_199_254_740_993_i64]}), &json!(9_007_199_254_740_993_i64); "integer past 2^53 exact")]
    fn integer_enum_valid(schema: &Value, instance: &Value) {
        tests_util::is_valid(schema, instance);
    }

    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(4); "integer rejects other integer")]
    #[test_case(&json!({"enum": [1, 2, 3]}), &json!("2"); "integer rejects string")]
    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(2.5); "integer rejects fraction")]
    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(null); "integer without null rejects null")]
    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(true); "integer rejects boolean")]
    #[test_case(&json!({"enum": [1]}), &json!(18_446_744_073_709_551_615_u64); "integer rejects u64 past i64")]
    #[test_case(&json!({"enum": [9_007_199_254_740_993_i64]}), &json!(9_007_199_254_740_992.0); "integer rejects float neighbour past 2^53")]
    #[test_case(&json!({"enum": [null, 10, 20]}), &json!(15); "integer with null rejects other integer")]
    fn integer_enum_invalid(schema: &Value, instance: &Value) {
        tests_util::is_not_valid(schema, instance);
    }

    // Options agreeing on length, first and last eight bytes share a head
    #[test_case("prefix0002suffix00", true)]
    #[test_case("prefix0003suffix00", false)]
    #[test_case("prefix0002suffix0", false)]
    fn colliding_string_options(instance: &str, expected: bool) {
        let schema = json!({"enum": ["prefix0001suffix00", "prefix0002suffix00"]});
        assert_eq!(crate::is_valid(&schema, &json!(instance)), expected);
    }

    #[test]
    fn big_string_enum_error_message() {
        let schema: Value = json_codec::from_str(BIG_STRING_ENUM).unwrap();
        tests_util::expect_errors(
            &schema,
            &json!("z"),
            &[r#""z" is not one of "a", "b" or 9 other candidates"#],
        );
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"enum": 5}),
            "5 is not of type \"array\"",
            "/enum",
        );
    }
}
