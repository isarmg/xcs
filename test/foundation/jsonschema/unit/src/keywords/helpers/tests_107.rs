#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"maxItems": -1}), "-1 is less than the minimum of 0", "/maxItems"; "max items negative")]
    #[test_case(&json!({"maxItems": "a"}), "\"a\" is not of type \"integer\"", "/maxItems"; "max items not integer")]
    #[test_case(&json!({"minItems": -1}), "-1 is less than the minimum of 0", "/minItems"; "min items negative")]
    #[test_case(&json!({"maxLength": -1}), "-1 is less than the minimum of 0", "/maxLength"; "max length negative")]
    #[test_case(&json!({"minLength": "a"}), "\"a\" is not of type \"integer\"", "/minLength"; "min length not integer")]
    #[test_case(&json!({"maxProperties": -1}), "-1 is less than the minimum of 0", "/maxProperties"; "max properties negative")]
    #[test_case(&json!({"minProperties": 1.5}), "1.5 is not of type \"integer\"", "/minProperties"; "min properties fractional")]
    #[test_case(&json!({"contains": {}, "minContains": -1}), "-1 is less than the minimum of 0", "/minContains"; "min contains negative")]
    #[test_case(&json!({"contains": {}, "maxContains": "a"}), "\"a\" is not of type \"integer\"", "/maxContains"; "max contains not integer")]
    fn malformed_size_limit(schema: &Value, message: &str, location: &str) {
        tests_util::assert_compile_error(schema, message, location);
    }
}
