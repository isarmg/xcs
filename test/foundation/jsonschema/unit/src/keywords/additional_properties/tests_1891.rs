#[cfg(test)]
mod tests {
    use crate::{properties::HASHMAP_THRESHOLD, tests_util, PatternOptions};
    use serde_json::{json, Map, Value};
    use test_case::test_case;

    // `properties` + `additionalProperties: false` + one `required` name. `a`..`e` are five
    // container members, one more than `is_valid` holds back.
    fn fused_required_schema() -> Value {
        json!({
            "additionalProperties": false,
            "required": ["id"],
            "properties": {
                "id": {"type": "integer"},
                "a": {"type": "array"},
                "b": {"type": "array"},
                "c": {"type": "array"},
                "d": {"type": "array"},
                "e": {"type": "array"},
            }
        })
    }

    #[test_case(&json!({"a": [], "b": [], "c": [], "d": [], "e": [], "id": 1}); "five containers")]
    #[test_case(&json!({"a": [], "id": 1}); "one container")]
    #[test_case(&json!({"id": 1}); "only the required member")]
    fn fused_required_valid(instance: &Value) {
        tests_util::is_valid(&fused_required_schema(), instance);
    }

    #[test_case(&json!({"a": [], "b": [], "c": [], "d": [], "e": {}, "id": 1}); "invalid container past the held-back ones")]
    #[test_case(&json!({"a": {}, "id": 1}); "invalid held-back container")]
    #[test_case(&json!({"a": [], "id": "x"}); "invalid scalar")]
    #[test_case(&json!({"a": [], "id": 1, "zzz": 1}); "undeclared member after a container")]
    #[test_case(&json!({"a": []}); "required member missing")]
    fn fused_required_invalid(instance: &Value) {
        tests_util::is_not_valid(&fused_required_schema(), instance);
    }

    fn schema_1() -> Value {
        // For `AdditionalPropertiesWithPatternsNotEmptyFalseValidator`
        json!({
            "additionalProperties": false,
            "properties": {
                "foo": {"type": "string"},
                "barbaz": {"type": "integer", "multipleOf": 3},
            },
            "patternProperties": {
                "^bar": {"type": "integer", "minimum": 5},
                "spam$": {"type": "integer", "maximum": 10},
            }
        })
    }

    // Another type
    #[test_case(&json!([1]))]
    // The right type
    #[test_case(&json!({}))]
    // Match `properties.foo`
    #[test_case(&json!({"foo": "a"}))]
    // Match `properties.barbaz` & `patternProperties.^bar`
    #[test_case(&json!({"barbaz": 6}))]
    // Match `patternProperties.^bar`
    #[test_case(&json!({"bar": 6}))]
    // Match `patternProperties.spam$`
    #[test_case(&json!({"spam": 7}))]
    // All `patternProperties` rules match on different values
    #[test_case(&json!({"bar": 6, "spam": 7}))]
    // All `patternProperties` rules match on the same value
    #[test_case(&json!({"barspam": 7}))]
    // All combined
    #[test_case(&json!({"barspam": 7, "bar": 6, "spam": 7, "foo": "a", "barbaz": 6}))]
    fn schema_1_valid(instance: &Value) {
        let schema = schema_1();
        tests_util::is_valid(&schema, instance);
    }

    // `properties.foo` - should be a string
    #[test_case(&json!({"foo": 3}), &["3 is not of type \"string\""], &["/properties/foo/type"])]
    // `additionalProperties` - extra keyword & not in `properties` / `patternProperties`
    #[test_case(&json!({"faz": 1}), &["Additional properties are not allowed (\'faz\' was unexpected)"], &["/additionalProperties"])]
    #[test_case(&json!({"faz": 1, "haz": 1}), &["Additional properties are not allowed (\'faz\', \'haz\' were unexpected)"], &["/additionalProperties"])]
    // `properties.foo` - should be a string & `patternProperties.^bar` - invalid
    #[test_case(&json!({"foo": 3, "bar": 4}), &["3 is not of type \"string\"","4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum","/properties/foo/type"])]
    // `properties.barbaz` - valid; `patternProperties.^bar` - invalid
    #[test_case(&json!({"barbaz": 3}), &["3 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` (should be >=5)
    #[test_case(&json!({"bar": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` (should be <=10)
    #[test_case(&json!({"spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - both values are invalid
    #[test_case(&json!({"bar": 4, "spam": 11}), &["11 is greater than the maximum of 10","4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum", "/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is valid, `spam` is invalid
    #[test_case(&json!({"bar": 6, "spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is invalid, `spam` is valid
    #[test_case(&json!({"bar": 4, "spam": 8}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` - (should be >=5), but valid for `patternProperties.spam$`
    #[test_case(&json!({"barspam": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` - (should be <=10), but valid for `patternProperties.^bar`
    #[test_case(&json!({"barspam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // All combined
    #[test_case(
      &json!({"bar": 4, "spam": 11, "foo": 3, "faz": 1}),
      &[
          "11 is greater than the maximum of 10",
          "3 is not of type \"string\"",
          "4 is less than the minimum of 5",
          "Additional properties are not allowed (\'faz\' was unexpected)",
      ],
      &[
          "/additionalProperties",
          "/patternProperties/^bar/minimum",
          "/patternProperties/spam$/maximum",
          "/properties/foo/type",
      ]
    )]
    fn schema_1_invalid(instance: &Value, expected: &[&str], locations: &[&str]) {
        let schema = schema_1();
        tests_util::is_not_valid(&schema, instance);
        tests_util::expect_errors(&schema, instance, expected);
        tests_util::assert_locations(&schema, instance, locations);
    }

    fn schema_2() -> Value {
        // For `AdditionalPropertiesWithPatternsFalseValidator`
        json!({
            "additionalProperties": false,
            "patternProperties": {
                "^bar": {"type": "integer", "minimum": 5},
                "spam$": {"type": "integer", "maximum": 10},
            }
        })
    }

    // Another type
    #[test_case(&json!([1]))]
    // The right type
    #[test_case(&json!({}))]
    // Match `patternProperties.^bar`
    #[test_case(&json!({"bar": 6}))]
    // Match `patternProperties.spam$`
    #[test_case(&json!({"spam": 7}))]
    // All `patternProperties` rules match on different values
    #[test_case(&json!({"bar": 6, "spam": 7}))]
    // All `patternProperties` rules match on the same value
    #[test_case(&json!({"barspam": 7}))]
    // All combined
    #[test_case(&json!({"barspam": 7, "bar": 6, "spam": 7}))]
    fn schema_2_valid(instance: &Value) {
        let schema = schema_2();
        tests_util::is_valid(&schema, instance);
    }

    // `additionalProperties` - extra keyword & not in `patternProperties`
    #[test_case(&json!({"faz": "a"}), &["Additional properties are not allowed (\'faz\' was unexpected)"], &["/additionalProperties"])]
    // `patternProperties.^bar` (should be >=5)
    #[test_case(&json!({"bar": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` (should be <=10)
    #[test_case(&json!({"spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - both values are invalid
    #[test_case(&json!({"bar": 4, "spam": 11}), &["11 is greater than the maximum of 10","4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum", "/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is valid, `spam` is invalid
    #[test_case(&json!({"bar": 6, "spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is invalid, `spam` is valid
    #[test_case(&json!({"bar": 4, "spam": 8}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` - (should be >=5), but valid for `patternProperties.spam$`
    #[test_case(&json!({"barspam": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` - (should be <=10), but valid for `patternProperties.^bar`
    #[test_case(&json!({"barspam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // All combined
    #[test_case(
      &json!({"bar": 4, "spam": 11, "faz": 1}),
      &[
          "11 is greater than the maximum of 10",
          "4 is less than the minimum of 5",
          "Additional properties are not allowed (\'faz\' was unexpected)",
      ],
      &[
          "/additionalProperties",
          "/patternProperties/^bar/minimum",
          "/patternProperties/spam$/maximum",
      ]
    )]
    fn schema_2_invalid(instance: &Value, expected: &[&str], locations: &[&str]) {
        let schema = schema_2();
        tests_util::is_not_valid(&schema, instance);
        tests_util::expect_errors(&schema, instance, expected);
        tests_util::assert_locations(&schema, instance, locations);
    }

    fn schema_3() -> Value {
        // For `AdditionalPropertiesNotEmptyFalseValidator`
        json!({
            "additionalProperties": false,
            "properties": {
                "foo": {"type": "string"}
            }
        })
    }

    // Another type
    #[test_case(&json!([1]))]
    // The right type
    #[test_case(&json!({}))]
    // Match `properties`
    #[test_case(&json!({"foo": "a"}))]
    fn schema_3_valid(instance: &Value) {
        let schema = schema_3();
        tests_util::is_valid(&schema, instance);
    }

    // `properties` - should be a string
    #[test_case(&json!({"foo": 3}), &["3 is not of type \"string\""], &["/properties/foo/type"])]
    // `additionalProperties` - extra keyword & not in `properties`
    #[test_case(&json!({"faz": "a"}), &["Additional properties are not allowed (\'faz\' was unexpected)"], &["/additionalProperties"])]
    // All combined
    #[test_case(
      &json!(
        {"foo": 3, "faz": "a"}),
        &[
            "3 is not of type \"string\"",
            "Additional properties are not allowed (\'faz\' was unexpected)",
        ],
        &[
            "/additionalProperties",
            "/properties/foo/type",
        ]
    )]
    fn schema_3_invalid(instance: &Value, expected: &[&str], locations: &[&str]) {
        let schema = schema_3();
        tests_util::is_not_valid(&schema, instance);
        tests_util::expect_errors(&schema, instance, expected);
        tests_util::assert_locations(&schema, instance, locations);
    }

    fn schema_4() -> Value {
        // For `AdditionalPropertiesNotEmptyValidator`
        json!({
            "additionalProperties": {"type": "integer"},
            "properties": {
                "foo": {"type": "string"}
            }
        })
    }

    // Another type
    #[test_case(&json!([1]))]
    // The right type
    #[test_case(&json!({}))]
    // Match `properties`
    #[test_case(&json!({"foo": "a"}))]
    // Match `additionalProperties`
    #[test_case(&json!({"bar": 4}))]
    // All combined
    #[test_case(&json!({"foo": "a", "bar": 4}))]
    fn schema_4_valid(instance: &Value) {
        let schema = schema_4();
        tests_util::is_valid(&schema, instance);
    }

    // `properties` - should be a string
    #[test_case(&json!({"foo": 3}), &["3 is not of type \"string\""], &["/properties/foo/type"])]
    // `additionalProperties` - should be an integer
    #[test_case(&json!({"bar": "a"}), &["\"a\" is not of type \"integer\""], &["/additionalProperties/type"])]
    // All combined
    #[test_case(
      &json!(
        {"foo": 3, "bar": "a"}),
        &[
            "\"a\" is not of type \"integer\"",
            "3 is not of type \"string\"",
        ],
        &[
            "/additionalProperties/type",
            "/properties/foo/type",
        ]
    )]
    fn schema_4_invalid(instance: &Value, expected: &[&str], locations: &[&str]) {
        let schema = schema_4();
        tests_util::is_not_valid(&schema, instance);
        tests_util::expect_errors(&schema, instance, expected);
        tests_util::assert_locations(&schema, instance, locations);
    }

    fn schema_5() -> Value {
        // For `AdditionalPropertiesWithPatternsNotEmptyValidator`
        json!({
            "additionalProperties": {"type": "integer"},
            "properties": {
                "foo": {"type": "string"},
                "barbaz": {"type": "integer", "multipleOf": 3},
            },
            "patternProperties": {
                "^bar": {"type": "integer", "minimum": 5},
                "spam$": {"type": "integer", "maximum": 10},
            }
        })
    }

    // Another type
    #[test_case(&json!([1]))]
    // The right type
    #[test_case(&json!({}))]
    // Match `properties.foo`
    #[test_case(&json!({"foo": "a"}))]
    // Match `additionalProperties`
    #[test_case(&json!({"faz": 42}))]
    // Match `properties.barbaz` & `patternProperties.^bar`
    #[test_case(&json!({"barbaz": 6}))]
    // Match `patternProperties.^bar`
    #[test_case(&json!({"bar": 6}))]
    // Match `patternProperties.spam$`
    #[test_case(&json!({"spam": 7}))]
    // All `patternProperties` rules match on different values
    #[test_case(&json!({"bar": 6, "spam": 7}))]
    // All `patternProperties` rules match on the same value
    #[test_case(&json!({"barspam": 7}))]
    // All combined
    #[test_case(&json!({"barspam": 7, "bar": 6, "spam": 7, "foo": "a", "barbaz": 6, "faz": 42}))]
    fn schema_5_valid(instance: &Value) {
        let schema = schema_5();
        tests_util::is_valid(&schema, instance);
    }

    // `properties.bar` - should be a string
    #[test_case(&json!({"foo": 3}), &["3 is not of type \"string\""], &["/properties/foo/type"])]
    // `additionalProperties` - extra keyword that doesn't match `additionalProperties`
    #[test_case(&json!({"faz": "a"}), &["\"a\" is not of type \"integer\""], &["/additionalProperties/type"])]
    #[test_case(&json!({"faz": "a", "haz": "a"}), &["\"a\" is not of type \"integer\"", "\"a\" is not of type \"integer\""], &["/additionalProperties/type", "/additionalProperties/type"])]
    // `properties.foo` - should be a string & `patternProperties.^bar` - invalid
    #[test_case(&json!({"foo": 3, "bar": 4}), &["3 is not of type \"string\"","4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum","/properties/foo/type"])]
    // `properties.barbaz` - valid; `patternProperties.^bar` - invalid
    #[test_case(&json!({"barbaz": 3}), &["3 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` (should be >=5)
    #[test_case(&json!({"bar": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` (should be <=10)
    #[test_case(&json!({"spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - both values are invalid
    #[test_case(&json!({"bar": 4, "spam": 11}), &["11 is greater than the maximum of 10","4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum", "/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is valid, `spam` is invalid
    #[test_case(&json!({"bar": 6, "spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is invalid, `spam` is valid
    #[test_case(&json!({"bar": 4, "spam": 8}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` - (should be >=5), but valid for `patternProperties.spam$`
    #[test_case(&json!({"barspam": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` - (should be <=10), but valid for `patternProperties.^bar`
    #[test_case(&json!({"barspam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // All combined + valid via `additionalProperties`
    #[test_case(
      &json!({"bar": 4, "spam": 11, "foo": 3, "faz": "a", "fam": 42}),
      &[
          "\"a\" is not of type \"integer\"",
          "11 is greater than the maximum of 10",
          "3 is not of type \"string\"",
          "4 is less than the minimum of 5",
      ],
      &[
          "/additionalProperties/type",
          "/patternProperties/^bar/minimum",
          "/patternProperties/spam$/maximum",
          "/properties/foo/type",
      ]
    )]
    fn schema_5_invalid(instance: &Value, expected: &[&str], locations: &[&str]) {
        let schema = schema_5();
        tests_util::is_not_valid(&schema, instance);
        tests_util::expect_errors(&schema, instance, expected);
        tests_util::assert_locations(&schema, instance, locations);
    }

    fn schema_6() -> Value {
        // For `AdditionalPropertiesWithPatternsValidator`
        json!({
            "additionalProperties": {"type": "integer"},
            "patternProperties": {
                "^bar": {"type": "integer", "minimum": 5},
                "spam$": {"type": "integer", "maximum": 10},
            }
        })
    }

    // Another type
    #[test_case(&json!([1]))]
    // The right type
    #[test_case(&json!({}))]
    // Match `additionalProperties`
    #[test_case(&json!({"faz": 42}))]
    // Match `patternProperties.^bar`
    #[test_case(&json!({"bar": 6}))]
    // Match `patternProperties.spam$`
    #[test_case(&json!({"spam": 7}))]
    // All `patternProperties` rules match on different values
    #[test_case(&json!({"bar": 6, "spam": 7}))]
    // All `patternProperties` rules match on the same value
    #[test_case(&json!({"barspam": 7}))]
    // All combined
    #[test_case(&json!({"barspam": 7, "bar": 6, "spam": 7, "faz": 42}))]
    fn schema_6_valid(instance: &Value) {
        let schema = schema_6();
        tests_util::is_valid(&schema, instance);
    }

    // `additionalProperties` - extra keyword that doesn't match `additionalProperties`
    #[test_case(&json!({"faz": "a"}), &["\"a\" is not of type \"integer\""], &["/additionalProperties/type"])]
    #[test_case(&json!({"faz": "a", "haz": "a"}), &["\"a\" is not of type \"integer\"", "\"a\" is not of type \"integer\""], &["/additionalProperties/type", "/additionalProperties/type"])]
    // `additionalProperties` - should be an integer & `patternProperties.^bar` - invalid
    #[test_case(&json!({"foo": "a", "bar": 4}), &["\"a\" is not of type \"integer\"","4 is less than the minimum of 5"], &["/additionalProperties/type","/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` (should be >=5)
    #[test_case(&json!({"bar": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` (should be <=10)
    #[test_case(&json!({"spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - both values are invalid
    #[test_case(&json!({"bar": 4, "spam": 11}), &["11 is greater than the maximum of 10","4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum", "/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is valid, `spam` is invalid
    #[test_case(&json!({"bar": 6, "spam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // `patternProperties` - `bar` is invalid, `spam` is valid
    #[test_case(&json!({"bar": 4, "spam": 8}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.^bar` - (should be >=5), but valid for `patternProperties.spam$`
    #[test_case(&json!({"barspam": 4}), &["4 is less than the minimum of 5"], &["/patternProperties/^bar/minimum"])]
    // `patternProperties.spam$` - (should be <=10), but valid for `patternProperties.^bar`
    #[test_case(&json!({"barspam": 11}), &["11 is greater than the maximum of 10"], &["/patternProperties/spam$/maximum"])]
    // All combined + valid via `additionalProperties`
    #[test_case(
      &json!({"bar": 4, "spam": 11, "faz": "a", "fam": 42}),
      &[
          "\"a\" is not of type \"integer\"",
          "11 is greater than the maximum of 10",
          "4 is less than the minimum of 5",
      ],
      &[
          "/additionalProperties/type",
          "/patternProperties/^bar/minimum",
          "/patternProperties/spam$/maximum",
      ]
    )]
    fn schema_6_invalid(instance: &Value, expected: &[&str], locations: &[&str]) {
        let schema = schema_6();
        tests_util::is_not_valid(&schema, instance);
        tests_util::expect_errors(&schema, instance, expected);
        tests_util::assert_locations(&schema, instance, locations);
    }

    // Test prefix optimization with additionalProperties: false
    #[test_case("^x-", "x-custom", true)]
    #[test_case("^x-", "y-other", false)]
    #[test_case("^eo_", "eo_bands", true)]
    #[test_case("^eo_", "other", false)]
    fn prefix_pattern_with_additional_false(pattern: &str, key: &str, matches_pattern: bool) {
        let schema = json!({
            "additionalProperties": false,
            "patternProperties": {
                pattern: {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Valid string value
        let string_instance = json!({ key: "value" });
        // If matches_pattern: key matches, string is valid type -> valid
        // If !matches_pattern: key is additional property, not allowed -> invalid
        assert_eq!(validator.is_valid(&string_instance), matches_pattern);

        // Invalid type (number instead of string)
        let number_instance = json!({ key: 42 });
        // If matches_pattern: key matches but wrong type -> invalid
        // If !matches_pattern: additional property not allowed -> invalid
        assert!(!validator.is_valid(&number_instance));
    }

    // Test prefix optimization with additionalProperties as schema
    #[test]
    fn prefix_pattern_with_additional_schema() {
        let schema = json!({
            "additionalProperties": {"type": "integer"},
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // x-foo matches pattern, must be string
        assert!(validator.is_valid(&json!({"x-foo": "bar"})));
        assert!(!validator.is_valid(&json!({"x-foo": 42})));

        // other doesn't match pattern, goes to additionalProperties (must be integer)
        assert!(validator.is_valid(&json!({"other": 42})));
        assert!(!validator.is_valid(&json!({"other": "str"})));

        // Both patterns
        assert!(validator.is_valid(&json!({"x-foo": "bar", "other": 42})));
        assert!(!validator.is_valid(&json!({"x-foo": 42, "other": "str"})));
    }

    // Test prefix optimization with properties + additionalProperties: false
    #[test]
    fn prefix_pattern_with_properties_and_additional_false() {
        let schema = json!({
            "properties": {
                "name": {"type": "string"}
            },
            "additionalProperties": false,
            "patternProperties": {
                "^x-": {"type": "string"}
            }
        });
        let validator = crate::validator_for(&schema).unwrap();

        // name is in properties
        assert!(validator.is_valid(&json!({"name": "test"})));
        assert!(!validator.is_valid(&json!({"name": 42})));

        // x-foo matches pattern
        assert!(validator.is_valid(&json!({"x-foo": "bar"})));
        assert!(!validator.is_valid(&json!({"x-foo": 42})));

        // other is neither in properties nor matches pattern -> additional, not allowed
        assert!(!validator.is_valid(&json!({"other": "value"})));

        // Combined valid
        assert!(validator.is_valid(&json!({"name": "test", "x-foo": "bar"})));
    }

    // Test fused validator with properties + additionalProperties: false + single required
    #[test]
    fn fused_additional_properties_false_with_required_1() {
        let schema = json!({
            "properties": {
                "id": {"type": "string"},
                "name": {"type": "string"}
            },
            "additionalProperties": false,
            "required": ["id"]
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Valid: has required and valid additional
        assert!(validator.is_valid(&json!({"id": "123"})));
        assert!(validator.is_valid(&json!({"id": "123", "name": "test"})));

        // Invalid: missing required
        assert!(!validator.is_valid(&json!({})));
        assert!(!validator.is_valid(&json!({"name": "test"})));

        // Invalid: has additional property
        assert!(!validator.is_valid(&json!({"id": "123", "extra": "val"})));

        // Invalid: wrong type for property
        assert!(!validator.is_valid(&json!({"id": 123})));

        // Non-object passes
        assert!(validator.is_valid(&json!("string")));
        assert!(validator.is_valid(&json!([1, 2, 3])));
    }

    #[test]
    fn fused_additional_properties_false_with_required_1_errors() {
        let schema = json!({
            "properties": {
                "id": {"type": "string"},
                "name": {"type": "string"}
            },
            "additionalProperties": false,
            "required": ["id"]
        });
        let validator = crate::validator_for(&schema).unwrap();

        // Missing required should produce required error
        let instance = json!({});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("required"));

        // Missing required with valid other property
        let instance = json!({"name": "test"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("required"));

        // Additional property error
        let instance = json!({"id": "123", "extra": "val"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("Additional properties"));

        // Both missing required and extra property
        let instance = json!({"extra": "val"});
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);
    }

    // `required` is inert, while `properties` and `additionalProperties` still apply.
    #[test_case(&json!({"properties": {"a": {}}, "additionalProperties": false, "required": ["b"]}); "one name")]
    #[test_case(&json!({"properties": {"a": {}}, "additionalProperties": false, "required": ["b", "c"]}); "two names")]
    fn additional_properties_without_validation_vocabulary(schema: &Value) {
        let instances = [json!({}), json!({"a": 1}), json!({"x": 1})];
        assert_eq!(
            tests_util::outcomes_with_only_vocabulary("applicator", schema, &instances),
            [
                (true, Vec::new()),
                (true, Vec::new()),
                (
                    false,
                    vec![(
                        "/additionalProperties".to_string(),
                        "Additional properties are not allowed ('x' was unexpected)".to_string()
                    )]
                ),
            ]
        );
    }

    const REQUIRED_A: &str = "\"a\" is a required property";
    const NOT_STRING: &str = "1 is not of type \"string\"";
    const X_UNEXPECTED: &str = "Additional properties are not allowed ('x' was unexpected)";

    #[test_case(&json!({}), &[(REQUIRED_A, "", "/required")]; "required alone")]
    #[test_case(
        &json!({"x": 1}),
        &[(X_UNEXPECTED, "", "/additionalProperties"), (REQUIRED_A, "", "/required")];
        "required with additional property"
    )]
    #[test_case(&json!({"a": 1}), &[(NOT_STRING, "/a", "/properties/a/type")]; "properties alone")]
    #[test_case(
        &json!({"a": 1, "x": 1}),
        &[(NOT_STRING, "/a", "/properties/a/type"), (X_UNEXPECTED, "", "/additionalProperties")];
        "properties with additional property"
    )]
    fn fused_required_error_locations(instance: &Value, expected: &[(&str, &str, &str)]) {
        tests_util::assert_error_locations(
            &json!({
                "properties": {"a": {"type": "string"}},
                "additionalProperties": false,
                "required": ["a"]
            }),
            instance,
            expected,
        );
    }

    #[test_case(
        &json!({"properties": {"a": {"type": "string"}}, "additionalProperties": false}),
        &json!({"a": 1, "x": 1}),
        &[(NOT_STRING, "/a", "/properties/a/type"), (X_UNEXPECTED, "", "/additionalProperties")];
        "properties with false"
    )]
    #[test_case(
        &json!({"properties": {"a": {"type": "string"}}, "additionalProperties": {"type": "string"}}),
        &json!({"a": 1, "x": 1}),
        &[(NOT_STRING, "/a", "/properties/a/type"), (NOT_STRING, "/x", "/additionalProperties/type")];
        "properties with subschema"
    )]
    #[test_case(
        &json!({"patternProperties": {"p": {"type": "string"}}, "additionalProperties": false}),
        &json!({"p": 1, "x": 1}),
        &[(NOT_STRING, "/p", "/patternProperties/p/type"), (X_UNEXPECTED, "", "/additionalProperties")];
        "patterns with false"
    )]
    #[test_case(
        &json!({"patternProperties": {"p": {"type": "string"}}, "additionalProperties": {"type": "string"}}),
        &json!({"p": 1, "x": 1}),
        &[(NOT_STRING, "/p", "/patternProperties/p/type"), (NOT_STRING, "/x", "/additionalProperties/type")];
        "patterns with subschema"
    )]
    #[test_case(
        &json!({
            "properties": {"a": {"type": "string"}},
            "patternProperties": {"p": {"type": "string"}},
            "additionalProperties": false
        }),
        &json!({"a": 1, "p": 1, "x": 1}),
        &[
            (NOT_STRING, "/a", "/properties/a/type"),
            (NOT_STRING, "/p", "/patternProperties/p/type"),
            (X_UNEXPECTED, "", "/additionalProperties"),
        ];
        "properties and patterns with false"
    )]
    #[test_case(
        &json!({
            "properties": {"a": {"type": "string"}},
            "patternProperties": {"p": {"type": "string"}},
            "additionalProperties": {"type": "string"}
        }),
        &json!({"a": 1, "p": 1, "x": 1}),
        &[
            (NOT_STRING, "/a", "/properties/a/type"),
            (NOT_STRING, "/p", "/patternProperties/p/type"),
            (NOT_STRING, "/x", "/additionalProperties/type"),
        ];
        "properties and patterns with subschema"
    )]
    fn absorbing_error_locations(
        schema: &Value,
        instance: &Value,
        expected: &[(&str, &str, &str)],
    ) {
        tests_util::assert_error_locations(schema, instance, expected);
    }

    const NOT_INTEGER: &str = "\"x\" is not of type \"integer\"";
    const EXTRA: &str = "Additional properties are not allowed ('extra' was unexpected)";

    /// Adds enough integer `properties` for them to be looked up in a hash map.
    fn with_many_properties(mut schema: Value) -> Value {
        let properties: Map<String, Value> = (0..HASHMAP_THRESHOLD)
            .map(|idx| (format!("p{idx}"), json!({"type": "integer"})))
            .collect();
        schema["properties"] = Value::Object(properties);
        schema
    }

    fn regex_engine_options() -> [crate::ValidationOptions<'static>; 2] {
        [
            crate::options().with_pattern_options(PatternOptions::fancy_regex()),
            crate::options().with_pattern_options(PatternOptions::regex()),
        ]
    }

    #[test_case(&with_many_properties(json!({"additionalProperties": false})), &json!({"p0": 1, "p14": 2}), None, &[]; "many properties and false valid")]
    #[test_case(
        &with_many_properties(json!({"additionalProperties": false})),
        &json!({"p0": "x", "extra": 1}),
        Some(EXTRA),
        &[(NOT_INTEGER, "/p0", "/properties/p0/type"), (EXTRA, "", "/additionalProperties")];
        "many properties and false invalid"
    )]
    #[test_case(&with_many_properties(json!({"additionalProperties": false, "required": ["p1"]})), &json!({"p1": 1}), None, &[]; "many properties, false and required valid")]
    #[test_case(
        &with_many_properties(json!({"additionalProperties": false, "required": ["p1"]})),
        &json!({"p0": "x", "extra": 1}),
        Some(EXTRA),
        &[
            (NOT_INTEGER, "/p0", "/properties/p0/type"),
            (EXTRA, "", "/additionalProperties"),
            ("\"p1\" is a required property", "", "/required"),
        ];
        "many properties, false and required invalid"
    )]
    #[test_case(&with_many_properties(json!({"additionalProperties": {"type": "string"}})), &json!({"p0": 1, "extra": "y"}), None, &[]; "many properties and schema valid")]
    #[test_case(
        &with_many_properties(json!({"additionalProperties": {"type": "string"}})),
        &json!({"p0": "x", "extra": 1}),
        Some(NOT_STRING),
        &[(NOT_STRING, "/extra", "/additionalProperties/type"), (NOT_INTEGER, "/p0", "/properties/p0/type")];
        "many properties and schema invalid"
    )]
    #[test_case(
        &with_many_properties(json!({"patternProperties": {"^x": {"type": "string"}}, "additionalProperties": false})),
        &json!({"p0": 1, "x1": "y"}),
        None,
        &[];
        "many properties, patterns and false valid"
    )]
    #[test_case(
        &with_many_properties(json!({"patternProperties": {"^x": {"type": "string"}}, "additionalProperties": false})),
        &json!({"p0": "x", "x1": 1, "extra": 1}),
        Some(EXTRA),
        &[
            (NOT_INTEGER, "/p0", "/properties/p0/type"),
            (NOT_STRING, "/x1", "/patternProperties/^x/type"),
            (EXTRA, "", "/additionalProperties"),
        ];
        "many properties, patterns and false invalid"
    )]
    #[test_case(
        &with_many_properties(json!({"patternProperties": {"^x": {"type": "string"}}, "additionalProperties": {"type": "string"}})),
        &json!({"p0": 1, "x1": "y", "extra": "z"}),
        None,
        &[];
        "many properties, patterns and schema valid"
    )]
    #[test_case(
        &with_many_properties(json!({"patternProperties": {"^x": {"type": "string"}}, "additionalProperties": {"type": "string"}})),
        &json!({"p0": "x", "x1": 1, "extra": 1}),
        Some(NOT_STRING),
        &[
            (NOT_STRING, "/extra", "/additionalProperties/type"),
            (NOT_INTEGER, "/p0", "/properties/p0/type"),
            (NOT_STRING, "/x1", "/patternProperties/^x/type"),
        ];
        "many properties, patterns and schema invalid"
    )]
    #[test_case(
        &json!({"patternProperties": {"^x": {"type": "string"}}, "additionalProperties": true}),
        &json!({"x1": 1, "extra": 1}),
        Some(NOT_STRING),
        &[(NOT_STRING, "/x1", "/patternProperties/^x/type")];
        "patterns and true"
    )]
    #[test_case(
        &json!({"patternProperties": {"^x": {"type": "string"}}, "additionalProperties": {"type": "string"}}),
        &json!({"x1": 1, "extra": 1}),
        Some(NOT_STRING),
        &[(NOT_STRING, "/extra", "/additionalProperties/type"), (NOT_STRING, "/x1", "/patternProperties/^x/type")];
        "patterns and schema"
    )]
    fn on_both_regex_engines(
        schema: &Value,
        instance: &Value,
        first: Option<&str>,
        expected: &[(&str, &str, &str)],
    ) {
        let expected: Vec<_> = expected
            .iter()
            .map(|(message, instance_path, schema_path)| {
                (
                    (*message).to_string(),
                    (*instance_path).to_string(),
                    (*schema_path).to_string(),
                )
            })
            .collect();
        for options in regex_engine_options() {
            let validator = options.build(schema).expect("Invalid schema");
            let errors: Vec<_> = validator
                .iter_errors(instance)
                .map(|error| {
                    (
                        error.to_string(),
                        error.instance_path().to_string(),
                        error.schema_path().to_string(),
                    )
                })
                .collect();
            assert_eq!(
                (
                    validator.is_valid(instance),
                    validator
                        .validate(instance)
                        .err()
                        .map(|error| error.to_string()),
                    errors
                ),
                (first.is_none(), first.map(str::to_string), expected.clone())
            );
        }
    }

    #[test_case(&json!({"additionalProperties": 5}), "5 is not of types \"boolean\", \"object\"", "/additionalProperties"; "not schema")]
    #[test_case(&json!({"patternProperties": 5, "additionalProperties": false}), "5 is not of type \"object\"", "/patternProperties"; "pattern properties not object")]
    #[test_case(&json!({"properties": 5, "patternProperties": {"a": {}}, "additionalProperties": false}), "Unexpected type", "/properties"; "properties not object with patterns and false")]
    #[test_case(&json!({"properties": 5, "patternProperties": {"a": {}}, "additionalProperties": {}}), "Unexpected type", "/properties"; "properties not object with patterns and schema")]
    fn malformed(schema: &Value, message: &str, location: &str) {
        for options in regex_engine_options() {
            tests_util::assert_compile_error_with(&options, schema, message, location);
        }
    }
}
