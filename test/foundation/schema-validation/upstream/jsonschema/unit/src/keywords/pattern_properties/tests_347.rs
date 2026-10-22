#[cfg(test)]
mod tests {
    use crate::{
        properties::HASHMAP_THRESHOLD,
        regex::{analyze_pattern, PatternOptimization},
        tests_util, PatternOptions,
    };
    use json_codec::{json, Map, Value};
    use test_case::test_case;

    #[test_case(&json!({"patternProperties": {"^f": {"type": "string"}}}), &json!({"f": 42}), "/patternProperties/^f/type")]
    #[test_case(&json!({"patternProperties": {"^f": {"type": "string"}, "^x": {"type": "string"}}}), &json!({"f": 42}), "/patternProperties/^f/type")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    #[derive(Clone, Copy)]
    enum Engine {
        FancyRegex,
        Regex,
    }

    fn with_many_properties(mut schema: Value) -> Value {
        let properties = (0..HASHMAP_THRESHOLD)
            .map(|idx| (format!("p{idx}"), json!({})))
            .collect::<Map<String, Value>>();
        schema["properties"] = Value::Object(properties);
        schema
    }

    const LOOKBEHIND: &str = "(?<=a)b";
    const UNBALANCED: &str = "a(";

    #[test_case(Engine::Regex, &json!({"patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex plain")]
    #[test_case(Engine::Regex, &json!({"patternProperties": {"^x": {}, LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex plain many")]
    #[test_case(Engine::Regex, &json!({"additionalProperties": true, "patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex additional true")]
    #[test_case(Engine::Regex, &json!({"additionalProperties": false, "patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex additional false")]
    #[test_case(Engine::Regex, &json!({"additionalProperties": {"type": "integer"}, "patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex additional schema")]
    #[test_case(Engine::Regex, &json!({"properties": {"foo": {}}, "additionalProperties": false, "patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex properties additional false")]
    #[test_case(Engine::Regex, &json!({"properties": {"foo": {}}, "additionalProperties": {"type": "integer"}, "patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex properties additional schema")]
    #[test_case(Engine::Regex, &with_many_properties(json!({"additionalProperties": false, "patternProperties": {LOOKBEHIND: {"type": "string"}}})), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex many properties additional false")]
    #[test_case(Engine::Regex, &with_many_properties(json!({"additionalProperties": {"type": "integer"}, "patternProperties": {LOOKBEHIND: {"type": "string"}}})), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex many properties additional schema")]
    #[test_case(Engine::Regex, &json!({"unevaluatedProperties": false, "patternProperties": {LOOKBEHIND: {"type": "string"}}}), LOOKBEHIND, "/patternProperties/(?<=a)b"; "regex unevaluated")]
    #[test_case(Engine::Regex, &json!({"patternProperties": {"a/b~(?<=c)": {}}}), "a/b~(?<=c)", "/patternProperties/a~1b~0(?<=c)"; "regex escaped key")]
    #[test_case(Engine::Regex, &json!({"additionalProperties": false, "patternProperties": {"a/b~(?<=c)": {}}}), "a/b~(?<=c)", "/patternProperties/a~1b~0(?<=c)"; "regex escaped key additional false")]
    #[test_case(Engine::Regex, &json!({"propertyNames": {"pattern": LOOKBEHIND}}), LOOKBEHIND, "/propertyNames/pattern"; "regex property names")]
    #[test_case(Engine::Regex, &json!({"pattern": LOOKBEHIND}), LOOKBEHIND, "/pattern"; "regex pattern")]
    #[test_case(Engine::FancyRegex, &json!({"patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy plain")]
    #[test_case(Engine::FancyRegex, &json!({"patternProperties": {"^x": {}, UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy plain many")]
    #[test_case(Engine::FancyRegex, &json!({"additionalProperties": true, "patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy additional true")]
    #[test_case(Engine::FancyRegex, &json!({"additionalProperties": false, "patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy additional false")]
    #[test_case(Engine::FancyRegex, &json!({"additionalProperties": {"type": "integer"}, "patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy additional schema")]
    #[test_case(Engine::FancyRegex, &json!({"properties": {"foo": {}}, "additionalProperties": false, "patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy properties additional false")]
    #[test_case(Engine::FancyRegex, &json!({"properties": {"foo": {}}, "additionalProperties": {"type": "integer"}, "patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy properties additional schema")]
    #[test_case(Engine::FancyRegex, &with_many_properties(json!({"additionalProperties": false, "patternProperties": {UNBALANCED: {"type": "string"}}})), UNBALANCED, "/patternProperties/a("; "fancy many properties additional false")]
    #[test_case(Engine::FancyRegex, &with_many_properties(json!({"additionalProperties": {"type": "integer"}, "patternProperties": {UNBALANCED: {"type": "string"}}})), UNBALANCED, "/patternProperties/a("; "fancy many properties additional schema")]
    #[test_case(Engine::FancyRegex, &json!({"unevaluatedProperties": false, "patternProperties": {UNBALANCED: {"type": "string"}}}), UNBALANCED, "/patternProperties/a("; "fancy unevaluated")]
    #[test_case(Engine::FancyRegex, &json!({"patternProperties": {"a/b~(": {}}}), "a/b~(", "/patternProperties/a~1b~0("; "fancy escaped key")]
    #[test_case(Engine::FancyRegex, &json!({"additionalProperties": false, "patternProperties": {"a/b~(": {}}}), "a/b~(", "/patternProperties/a~1b~0("; "fancy escaped key additional false")]
    #[test_case(Engine::FancyRegex, &json!({"propertyNames": {"pattern": UNBALANCED}}), UNBALANCED, "/propertyNames/pattern"; "fancy property names")]
    #[test_case(Engine::FancyRegex, &json!({"pattern": UNBALANCED}), UNBALANCED, "/pattern"; "fancy pattern")]
    fn invalid_regex(engine: Engine, schema: &Value, pattern: &str, schema_path: &str) {
        let options = crate::options();
        let error = match engine {
            Engine::FancyRegex => options
                .with_pattern_options(PatternOptions::fancy_regex())
                .build(schema),
            Engine::Regex => options
                .with_pattern_options(PatternOptions::regex())
                .build(schema),
        }
        .expect_err("Should fail to compile");
        assert_eq!(
            (
                error.to_string(),
                error.instance().as_ref(),
                error.schema_path().as_str(),
                error.evaluation_path().as_str(),
            ),
            (
                format!("{} is not a \"regex\"", json!(pattern)),
                &json!(pattern),
                schema_path,
                schema_path,
            )
        );
    }

    #[test]
    fn test_analyze_pattern() {
        use PatternOptimization::{Exact, Prefix};
        assert_eq!(analyze_pattern("^foo"), Some(Prefix("foo".into())));
        assert_eq!(analyze_pattern("^x-"), Some(Prefix("x-".into())));
        assert_eq!(analyze_pattern("^eo_band"), Some(Prefix("eo_band".into())));
        assert_eq!(analyze_pattern("^path/to"), Some(Prefix("path/to".into())));
        assert_eq!(analyze_pattern("^ABC123"), Some(Prefix("ABC123".into())));
        assert_eq!(analyze_pattern("^\\/"), Some(Prefix("/".into())));
        assert_eq!(analyze_pattern("^foo$"), Some(Exact("foo".into())));
        assert_eq!(analyze_pattern("^\\$ref$"), Some(Exact("$ref".into())));
        assert_eq!(analyze_pattern("foo"), None);
        assert_eq!(analyze_pattern("^foo.*"), None);
        assert_eq!(analyze_pattern("^foo+"), None);
        assert_eq!(analyze_pattern("^foo?"), None);
        assert_eq!(analyze_pattern("^[a-z]"), None);
        assert_eq!(analyze_pattern("^foo|bar"), None);
        assert_eq!(analyze_pattern("^foo(bar)"), None);
        assert_eq!(analyze_pattern("^foo\\d"), None);
    }

    // Test that prefix optimization works correctly for validation
    #[test_case("^x-", "x-custom", true)]
    #[test_case("^x-", "custom", false)]
    #[test_case("^eo_", "eo_bands", true)]
    #[test_case("^eo_", "proj_epsg", false)]
    fn test_prefix_pattern_validation(pattern: &str, key: &str, should_match: bool) {
        let schema = json!({
            "patternProperties": {
                pattern: {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // If key matches pattern, value must be string
        let valid_instance = json!({ key: "value" });
        assert!(validator.is_valid(&valid_instance));

        let invalid_instance = json!({ key: 42 });
        assert_eq!(validator.is_valid(&invalid_instance), !should_match);
    }

    // Test multiple prefix patterns
    #[test]
    fn test_multiple_prefix_patterns() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"},
                "^y-": {"type": "number"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        assert!(validator.is_valid(&json!({"x-foo": "bar", "y-baz": 42})));
        assert!(!validator.is_valid(&json!({"x-foo": 123}))); // x- must be string
        assert!(!validator.is_valid(&json!({"y-baz": "str"}))); // y- must be number
    }

    // iter_errors tests for prefix patterns
    #[test]
    fn test_prefix_iter_errors_valid() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Valid: no errors
        let instance = json!({"x-foo": "bar"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert!(errors.is_empty());

        // Valid: non-matching key is ignored
        let instance = json!({"other": 42});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_prefix_iter_errors_invalid() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Invalid: wrong type
        let instance = json!({"x-foo": 42});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("not of type"));
    }

    #[test]
    fn test_prefix_iter_errors_multiple_failures() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"},
                "^y-": {"type": "number"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Multiple errors
        let instance = json!({"x-a": 1, "y-b": "str"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);
    }

    // evaluate tests for prefix patterns
    #[test]
    fn test_prefix_evaluate_valid() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        let instance = json!({"x-foo": "bar"});
        let result = validator.evaluate(&instance);
        assert!(result.flag().valid);
    }

    #[test]
    fn test_prefix_evaluate_invalid() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        let instance = json!({"x-foo": 42});
        let result = validator.evaluate(&instance);
        assert!(!result.flag().valid);
    }

    #[test]
    fn test_prefix_evaluate_annotations() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Valid case should have annotations for matched properties
        let instance = json!({"x-foo": "bar", "x-baz": "qux", "other": 123});
        let result = validator.evaluate(&instance);
        assert!(result.flag().valid);

        // Check annotations exist
        let annotations: Vec<_> = result.iter_annotations().collect();
        assert!(!annotations.is_empty());
    }

    #[test]
    fn test_prefix_multiple_patterns_evaluate() {
        let schema = json!({
            "patternProperties": {
                "^x-": {"type": "string"},
                "^y-": {"type": "number"},
                "^z$": {"type": "boolean"},
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // All valid
        let instance = json!({"x-a": "s", "y-b": 1, "z": true});
        let result = validator.evaluate(&instance);
        assert!(result.flag().valid);

        // One invalid
        let instance = json!({"x-a": 123});
        let result = validator.evaluate(&instance);
        assert!(!result.flag().valid);
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"patternProperties": 5}),
            "5 is not of type \"object\"",
            "/patternProperties",
        );
    }
}
