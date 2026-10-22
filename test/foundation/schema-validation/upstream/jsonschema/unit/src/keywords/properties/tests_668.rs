#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::{json, Map, Value};
    use test_case::test_case;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"properties": {"foo": {"properties": {"bar": {"required": ["spam"]}}}}}),
            &json!({"foo": {"bar": {}}}),
            "/properties/foo/properties/bar/required",
        );
    }

    // SmallPropertiesWithRequired2Validator tests
    fn fused_schema() -> Value {
        // No additionalProperties: false, so uses SmallPropertiesWithRequired2Validator
        json!({
            "properties": {
                "a": {"type": "integer"},
                "b": {"type": "string"}
            },
            "required": ["a", "b"]
        })
    }

    #[test_case(&json!({"a": 1, "b": "x"}), true)]
    #[test_case(&json!({"a": 1, "b": "x", "c": 3}), true)]
    #[test_case(&json!({"a": 1}), false)] // missing b
    #[test_case(&json!({"b": "x"}), false)] // missing a
    #[test_case(&json!({}), false)]
    #[test_case(&json!("string"), true)] // non-object passes
    fn fused_properties_required2_is_valid(instance: &Value, expected: bool) {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
    }

    // Declared names and instance keys that agree on length and on long prefixes
    #[test_case("abcdefghijklX", "abcdefghijklX", true; "thirteen bytes equal")]
    #[test_case("abcdefghijklX", "abcdefghijklY", false; "thirteen bytes differing after the head")]
    #[test_case("abcdefghijkl", "abcdefghijkl", true; "twelve bytes equal")]
    #[test_case("abcdefghijkl", "abcdefghijkL", false; "twelve bytes differing in the last byte")]
    #[test_case("abcdEFGHijkl", "abcdefghijkl", false; "twelve bytes differing in the middle")]
    #[test_case("abcd", "abcd", true; "four bytes equal")]
    #[test_case("abcd", "abce", false; "four bytes differing")]
    #[test_case("abcd", "abcde", false; "prefix of a longer key")]
    #[test_case("", "", true; "empty equal")]
    #[test_case("", "a", false; "empty against one byte")]
    #[test_case("café-résumé-naïve", "café-résumé-naïve", true; "multibyte equal")]
    #[test_case("café-résumé-naïve", "café-rÉsumé-naïve", false; "multibyte differing after the head")]
    fn property_name_matching(declared: &str, key: &str, matches: bool) {
        let instance = json!({key: 1, "p": 1});
        let scanned = json!({"properties": {declared: false, "p": true}});
        assert_eq!(crate::is_valid(&scanned, &instance), !matches);
        let fused =
            json!({"properties": {declared: false, "p": true}, "required": [declared, "p"]});
        assert!(!crate::is_valid(&fused, &instance));
        let fused_present =
            json!({"properties": {declared: true, "p": true}, "required": [declared, "p"]});
        assert_eq!(crate::is_valid(&fused_present, &instance), matches);
        let additional = json!({
            "properties": {declared: {"type": "integer"}},
            "additionalProperties": {"type": "string"},
        });
        assert_eq!(crate::is_valid(&additional, &json!({key: 1})), matches);
    }

    // Names agreeing on length, first and last eight bytes share a head
    fn colliding_name(middle: usize) -> String {
        format!("prefix00{middle:02}suffix00")
    }

    #[test_case("plain", 3, 3, true)]
    #[test_case("plain", 3, 4, false)]
    #[test_case("plain", 13, 13, true)]
    #[test_case("plain", 0, 13, false)]
    #[test_case("plain", 99, 0, true)]
    #[test_case("fused", 3, 3, true)]
    #[test_case("fused", 3, 4, false)]
    #[test_case("fused", 13, 13, true)]
    #[test_case("additional", 3, 3, true)]
    #[test_case("additional", 3, 4, false)]
    #[test_case("additional", 99, 0, false)]
    fn colliding_heads_select_the_named_property(
        kind: &str,
        key: usize,
        value: usize,
        expected: bool,
    ) {
        let properties: Map<String, Value> = (0..14)
            .map(|index| (colliding_name(index), json!({"const": index})))
            .collect();
        let mut schema = json!({"properties": properties});
        let mut instance = json!({colliding_name(key): value});
        match kind {
            "fused" => {
                schema["required"] = json!([colliding_name(0), colliding_name(1)]);
                instance[colliding_name(0)] = json!(0);
                instance[colliding_name(1)] = json!(1);
            }
            "additional" => schema["additionalProperties"] = json!(false),
            _ => {}
        }
        assert_eq!(crate::is_valid(&schema, &instance), expected);
    }

    // Required names outside `properties`
    #[test_case(&json!({"a": 1, "z": 2}), true)]
    #[test_case(&json!({"a": 1}), false)]
    #[test_case(&json!({"z": 2, "y": 3}), false)]
    #[test_case(&json!({"a": "x", "z": 2}), false)]
    fn fused_required_outside_properties(instance: &Value, expected: bool) {
        let schema = json!({
            "properties": {"a": {"type": "integer"}, "b": {}},
            "required": ["a", "z"],
        });
        assert_eq!(crate::is_valid(&schema, instance), expected);
        assert_eq!(
            crate::validator_for(&schema)
                .unwrap()
                .validate(instance)
                .is_ok(),
            expected
        );
    }

    // A missing required name is reported before an invalid property
    #[test_case(&json!({"a": "x"}), r#""b" is a required property"#)]
    #[test_case(&json!({"b": 1}), r#""a" is a required property"#)]
    #[test_case(&json!({"a": "x", "b": "y"}), r#""x" is not of type "integer""#)]
    fn fused_error_precedence(instance: &Value, expected: &str) {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        assert_eq!(
            validator.validate(instance).unwrap_err().to_string(),
            expected
        );
    }

    #[test]
    fn fused_properties_required2_validate_missing_first() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let instance = json!({"b": "x"});
        let result = validator.validate(&instance);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("required"));
    }

    #[test]
    fn fused_properties_required2_validate_missing_second() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let instance = json!({"a": 1});
        let result = validator.validate(&instance);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("required"));
    }

    #[test]
    fn fused_properties_required2_iter_errors_missing_both() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let instance = json!({});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn fused_properties_required2_iter_errors_missing_first() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let instance = json!({"b": "x"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn fused_properties_required2_iter_errors_missing_second() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let instance = json!({"a": 1});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn fused_properties_required2_iter_errors_valid() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let instance = json!({"a": 1, "b": "x"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert!(errors.is_empty());
    }

    #[test_case(&json!({"a": 1, "b": "x"}), &[])] // valid
    #[test_case(&json!({"a": 1}), &[("/required", "", "required", "\"b\" is a required property")])] // missing b
    #[test_case(&json!({"b": "x"}), &[("/required", "", "required", "\"a\" is a required property")])] // missing a
    #[test_case(&json!({}), &[
        ("/required", "", "required", "\"a\" is a required property"),
        ("/required", "", "required", "\"b\" is a required property"),
    ])] // missing both
    fn fused_properties_required2_evaluate(
        instance: &Value,
        expected: &[(&str, &str, &str, &str)],
    ) {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        let eval = validator.evaluate(instance);
        assert_eq!(eval.flag().valid, expected.is_empty());
        let errors: Vec<_> = eval
            .iter_errors()
            .map(|e| {
                (
                    e.schema_location,
                    e.instance_location.as_str(),
                    e.error.keyword(),
                    e.error.message(),
                )
            })
            .collect();
        assert_eq!(errors.as_slice(), expected);
    }

    fn two_props_schema() -> Value {
        json!({"properties": {"a": {"type": "integer"}, "b": {"type": "string"}}})
    }

    fn with_extra_keys(base: Value, count: usize) -> Value {
        let Value::Object(mut map) = base else {
            unreachable!()
        };
        for i in 0..count {
            map.insert(format!("extra{i}"), json!(i));
        }
        Value::Object(map)
    }

    // len(instance) <= declared props -> single-pass iterate branch
    #[test_case(&json!({"a": 1, "b": "x"}), true)]
    #[test_case(&json!({"a": 1}), true)] // subset of declared props
    #[test_case(&json!({"a": "not-int", "b": "x"}), false)]
    #[test_case(&json!({"b": 2}), false)] // b must be string
    fn small_properties_iterate_branch(instance: &Value, expected: bool) {
        let validator = crate::validator_for(&two_props_schema()).unwrap();
        assert_eq!(validator.is_valid(instance), expected);
    }

    // len(instance) > declared props -> targeted-lookup get branch
    #[test_case(json!({"a": 1, "b": "x"}), true)]
    #[test_case(json!({"a": "not-int", "b": "x"}), false)]
    fn small_properties_get_branch_wide_instance(base: Value, expected: bool) {
        let validator = crate::validator_for(&two_props_schema()).unwrap();
        let instance = with_extra_keys(base, 300);
        assert_eq!(validator.is_valid(&instance), expected);
    }

    // validate/iter_errors reach the same verdict on both crossover branches.
    #[test_case(json!({"a": 1, "b": "x"}), true)] // small -> iterate branch
    #[test_case(json!({"a": "not-int", "b": "x"}), false)]
    fn small_properties_validate_iter_errors_crossover(base: Value, valid: bool) {
        let validator = crate::validator_for(&two_props_schema()).unwrap();
        // small instance (iterate branch)
        assert_eq!(validator.validate(&base).is_ok(), valid);
        assert_eq!(validator.iter_errors(&base).next().is_none(), valid);
        // same instance widened past the declared-prop count (get branch)
        let wide = with_extra_keys(base, 300);
        assert_eq!(validator.validate(&wide).is_ok(), valid);
        assert_eq!(validator.iter_errors(&wide).next().is_none(), valid);
    }

    // Fused variant: a missing required key fails on both crossover branches.
    #[test]
    fn fused_required_missing_both_branches() {
        let validator = crate::validator_for(&fused_schema()).unwrap();
        // len == declared props, required "b" absent -> iterate branch (seen_second == false)
        assert!(!validator.is_valid(&json!({"a": 1, "c": 3})));
        // wide instance, required "a" absent -> get branch early fast-fail
        let wide = with_extra_keys(json!({"b": "x"}), 300);
        assert!(!validator.is_valid(&wide));
    }

    #[test]
    fn fused_required_absolute_keyword_locations() {
        tests_util::assert_absolute_keyword_locations(
            &json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$id": "https://example.com/s.json",
                "properties": {"a": {"type": "string"}},
                "required": ["a", "b"]
            }),
            &json!({}),
            &[
                ("required", "https://example.com/s.json#/required"),
                ("required", "https://example.com/s.json#/required"),
            ],
        );
    }

    // `required` is inert, while `properties` still applies.
    #[test_case(&json!({"properties": {"a": false}, "required": ["b", "c"]}); "two names")]
    #[test_case(&json!({"properties": {"a": false}, "required": ["b", "c", "d"]}); "three names")]
    fn properties_without_validation_vocabulary(schema: &Value) {
        let instances = [json!({}), json!({"a": 1})];
        assert_eq!(
            tests_util::outcomes_with_only_vocabulary("applicator", schema, &instances),
            [
                (true, Vec::new()),
                (
                    false,
                    vec![(
                        "/properties/a".to_string(),
                        "False schema does not allow 1".to_string()
                    )]
                ),
            ]
        );
    }

    #[test_case(
        &json!({}),
        &[
            ("\"a\" is a required property", "", "/required"),
            ("\"b\" is a required property", "", "/required"),
        ];
        "required alone"
    )]
    #[test_case(
        &json!({"a": 1}),
        &[
            ("\"b\" is a required property", "", "/required"),
            ("1 is not of type \"string\"", "/a", "/properties/a/type"),
        ];
        "required with properties"
    )]
    #[test_case(
        &json!({"a": 1, "b": 1}),
        &[("1 is not of type \"string\"", "/a", "/properties/a/type")];
        "properties alone"
    )]
    fn fused_required_error_locations(instance: &Value, expected: &[(&str, &str, &str)]) {
        tests_util::assert_error_locations(
            &json!({"properties": {"a": {"type": "string"}}, "required": ["a", "b"]}),
            instance,
            expected,
        );
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"properties": 5}),
            "5 is not of type \"object\"",
            "/properties",
        );
    }
}
