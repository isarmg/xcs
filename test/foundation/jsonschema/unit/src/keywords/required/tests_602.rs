#[cfg(test)]
mod tests {
    use super::HASHMAP_THRESHOLD;
    use crate::tests_util;
    use json_codec::{json, Map, Value};
    use test_case::test_case;

    #[test_case(&json!({"required": ["a"]}), &json!({}), "/required")]
    #[test_case(&json!({"required": ["a", "b"]}), &json!({}), "/required")]
    #[test_case(&json!({"required": ["a", "b", "c"]}), &json!({}), "/required")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    #[test_case(&json!({"required": 5}), "5 is not of type \"array\""; "not array")]
    #[test_case(&json!({"required": [1]}), "1 is not of type \"string\""; "one")]
    #[test_case(&json!({"required": ["a", 1]}), "1 is not of type \"string\""; "two second")]
    #[test_case(&json!({"required": [1, "a"]}), "1 is not of type \"string\""; "two first")]
    #[test_case(&json!({"required": ["a", "b", 1]}), "1 is not of type \"string\""; "three third")]
    #[test_case(&json!({"required": ["a", 1, "c"]}), "1 is not of type \"string\""; "three second")]
    #[test_case(&json!({"required": [1, "b", "c"]}), "1 is not of type \"string\""; "three first")]
    #[test_case(&json!({"required": ["a", "b", "c", 1]}), "1 is not of type \"string\""; "many")]
    fn malformed(schema: &Value, message: &str) {
        tests_util::assert_compile_error(schema, message, "/required");
    }

    // Required names and instance keys that agree on length and on long prefixes
    #[test_case("abcdefghijklX", "abcdefghijklX", true; "thirteen bytes equal")]
    #[test_case("abcdefghijklX", "abcdefghijklY", false; "thirteen bytes differing after the head")]
    #[test_case("abcdefghijkl", "abcdefghijkL", false; "twelve bytes differing in the last byte")]
    #[test_case("abcdEFGHijkl", "abcdefghijkl", false; "twelve bytes differing in the middle")]
    #[test_case("abcd", "abce", false; "four bytes differing")]
    #[test_case("abcd", "abcde", false; "prefix of a longer key")]
    #[test_case("", "", true; "empty equal")]
    #[test_case("café-résumé-naïve", "café-rÉsumé-naïve", false; "multibyte differing after the head")]
    fn required_name_matching(required: &str, key: &str, matches: bool) {
        let instance = json!({key: 1, "p": 1, "q": 1, "r": 1});
        let three = json!({"required": [required, "p", "q"]});
        assert_eq!(crate::is_valid(&three, &instance), matches);
        let four = json!({"required": [required, "p", "q", "r"]});
        assert_eq!(crate::is_valid(&four, &instance), matches);
    }

    // Required2Validator tests
    #[test_case(&json!({"a": 1, "b": 2}), true)]
    #[test_case(&json!({"a": 1, "b": 2, "c": 3}), true)]
    #[test_case(&json!({"a": 1}), false)]
    #[test_case(&json!({"b": 2}), false)]
    #[test_case(&json!({}), false)]
    #[test_case(&json!([1, 2]), true)] // Non-object passes
    fn required_2(instance: &Value, expected: bool) {
        let schema = json!({"required": ["a", "b"]});
        let validator = crate::validator_for(&schema).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
    }

    // Required3Validator tests
    #[test_case(&json!({"a": 1, "b": 2, "c": 3}), true)]
    #[test_case(&json!({"a": 1, "b": 2, "c": 3, "d": 4}), true)]
    #[test_case(&json!({"a": 1, "b": 2}), false)]
    #[test_case(&json!({"a": 1, "c": 3}), false)]
    #[test_case(&json!({"b": 2, "c": 3}), false)]
    #[test_case(&json!({}), false)]
    #[test_case(&json!("string"), true)] // Non-object passes
    fn required_3(instance: &Value, expected: bool) {
        let schema = json!({"required": ["a", "b", "c"]});
        let validator = crate::validator_for(&schema).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
    }

    // Objects wider than the scan window take the lookup branch instead.
    #[test_case(&json!({"a": 1, "b": 2, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7}), true)]
    #[test_case(&json!({"b": 2, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8}), false)]
    #[test_case(&json!({"a": 1, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8}), false)]
    #[test_case(&json!({"a": 1, "b": 2, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8}), false)]
    fn required_3_wide(instance: &Value, expected: bool) {
        let schema = json!({"required": ["a", "b", "c"]});
        let validator = crate::validator_for(&schema).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
    }

    #[test_case(&json!({"b": 2, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8}), r#""a" is a required property"#)]
    #[test_case(&json!({"a": 1, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8}), r#""b" is a required property"#)]
    #[test_case(&json!({"a": 1, "b": 2, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8}), r#""c" is a required property"#)]
    fn required_3_wide_validate(instance: &Value, expected: &str) {
        let schema = json!({"required": ["a", "b", "c"]});
        let validator = crate::validator_for(&schema).unwrap();
        let error = validator.validate(instance).expect_err("Should fail");
        assert_eq!(error.to_string(), expected);
    }

    #[test]
    fn required_3_wide_iter_errors() {
        let schema = json!({"required": ["a", "b", "c"]});
        let validator = crate::validator_for(&schema).unwrap();
        let instance = json!({"a": 1, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].to_string(), r#""b" is a required property"#);
    }

    #[test_case(&json!({"a": 1, "b": 2, "c": 3, "d": 4, "e": 5, "f": 6, "g": 7, "h": 8, "i": 9}), true)]
    #[test_case(&json!({"a": 1, "b": 2, "c": 3, "e": 5, "f": 6, "g": 7, "h": 8, "i": 9, "j": 10}), false)]
    fn required_many_wide(instance: &Value, expected: bool) {
        let schema = json!({"required": ["a", "b", "c", "d"]});
        let validator = crate::validator_for(&schema).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
    }

    #[test]
    fn required_many_wide_errors() {
        let schema = json!({"required": ["a", "b", "c", "d"]});
        let validator = crate::validator_for(&schema).unwrap();
        let instance =
            json!({"a": 1, "c": 3, "e": 5, "f": 6, "g": 7, "h": 8, "i": 9, "j": 10, "k": 11});
        let error = validator.validate(&instance).expect_err("Should fail");
        assert_eq!(error.to_string(), r#""b" is a required property"#);
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].to_string(), r#""b" is a required property"#);
        assert_eq!(errors[1].to_string(), r#""d" is a required property"#);
    }

    #[test]
    fn required_2_iter_errors() {
        let schema = json!({"required": ["a", "b"]});
        let validator = crate::validator_for(&schema).unwrap();

        // Missing both
        let instance = json!({});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);

        // Missing one
        let instance = json!({"a": 1});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);

        // All present
        let instance = json!({"a": 1, "b": 2});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn required_3_iter_errors() {
        let schema = json!({"required": ["a", "b", "c"]});
        let validator = crate::validator_for(&schema).unwrap();

        // Missing all
        let instance = json!({});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 3);

        // Missing two
        let instance = json!({"a": 1});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);

        // Missing one
        let instance = json!({"a": 1, "b": 2});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);

        // All present
        let instance = json!({"a": 1, "b": 2, "c": 3});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert!(errors.is_empty());
    }

    // When `additionalProperties` is a schema object, properties::compile returns None so
    // SmallPropertiesWithRequired2Validator is never created; required must still be enforced.
    #[test]
    fn required_2_enforced_with_additional_properties_schema() {
        let schema = json!({
            "properties": {
                "type": {"type": "string"},
                "linkedServiceName": {"type": "object"},
            },
            "additionalProperties": {"type": "object"},
            "required": ["type", "linkedServiceName"],
        });
        let validator = crate::validator_for(&schema).unwrap();

        assert!(!validator.is_valid(&json!({"type": "x"})));
        assert!(!validator.is_valid(&json!({"linkedServiceName": {}})));
        assert!(!validator.is_valid(&json!({})));
        assert!(validator.is_valid(&json!({"type": "x", "linkedServiceName": {}})));
    }

    // When `properties` has >= HASHMAP_THRESHOLD entries the fused SmallPropertiesWithRequired2
    // is not used; the standalone required validator must still fire.
    #[test]
    fn required_2_enforced_with_large_properties_map() {
        let mut props = json_codec::Map::new();
        for i in 0..HASHMAP_THRESHOLD {
            props.insert(format!("prop{i}"), json!({"type": "string"}));
        }
        props.insert("vmSize".to_string(), json!({"type": "string"}));
        props.insert("count".to_string(), json!({"type": "integer"}));

        let schema = json!({
            "properties": Value::Object(props),
            "required": ["vmSize", "count"]
        });
        let validator = crate::validator_for(&schema).unwrap();

        assert!(!validator.is_valid(&json!({"count": 1})));
        assert!(!validator.is_valid(&json!({"vmSize": "x"})));
        assert!(validator.is_valid(&json!({"vmSize": "x", "count": 1})));

        let instance = json!({"count": 1});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
    }

    // Sixteen names agreeing on length, first and last eight bytes share a head
    #[test_case(16, true)]
    #[test_case(15, false)]
    fn colliding_required_names_scanned(present: usize, expected: bool) {
        let names: Vec<String> = (0..16).map(|i| format!("prefix00{i:02}suffix00")).collect();
        let schema = json!({"required": names});
        let instance: Map<String, Value> = names
            .iter()
            .take(present)
            .map(|name| (name.clone(), json!(1)))
            .collect();
        assert_eq!(crate::is_valid(&schema, &Value::Object(instance)), expected);
    }

    fn missing(names: &[&str]) -> (bool, Vec<(String, String)>) {
        let errors = names
            .iter()
            .map(|name| {
                (
                    "/required".to_string(),
                    format!("\"{name}\" is a required property"),
                )
            })
            .collect();
        (false, errors)
    }

    // `properties` and `additionalProperties` are inert, while `required` still applies.
    #[test_case(&json!({"properties": {"a": false}, "required": ["a", "b"]}), &["a", "b"]; "properties two names")]
    #[test_case(&json!({"properties": {"a": false}, "additionalProperties": false, "required": ["b"]}), &["b"]; "properties additional false one name")]
    #[test_case(&json!({"properties": {"a": false}, "required": ["a", "b", "c"]}), &["a", "b", "c"]; "properties three names")]
    #[test_case(&json!({"required": ["a", "b"]}), &["a", "b"]; "required alone")]
    fn required_without_applicator_vocabulary(schema: &Value, expected_missing: &[&str]) {
        let instances = [json!({}), json!({"a": 1, "b": 1, "c": 1})];
        assert_eq!(
            tests_util::outcomes_with_only_vocabulary("validation", schema, &instances),
            [missing(expected_missing), (true, Vec::new())]
        );
    }
}
