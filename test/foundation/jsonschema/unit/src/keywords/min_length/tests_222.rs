#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(&json!({"minLength": 1}), &json!(""), "/minLength");
    }

    #[test]
    fn fused_locations() {
        let schema = json!({"minLength": 2, "maxLength": 4});
        tests_util::assert_schema_location(&schema, &json!("a"), "/minLength");
        tests_util::assert_schema_location(&schema, &json!("abcde"), "/maxLength");
    }

    #[test]
    fn fused_bounds_are_inclusive() {
        let schema = json!({"minLength": 2, "maxLength": 4});
        tests_util::is_valid(&schema, &json!("ab"));
        tests_util::is_valid(&schema, &json!("abcd"));
        tests_util::is_not_valid(&schema, &json!("a"));
        tests_util::is_not_valid(&schema, &json!("abcde"));
    }

    #[test]
    fn fused_ignores_non_strings() {
        let schema = json!({"minLength": 2, "maxLength": 4});
        tests_util::is_valid(&schema, &json!(1));
        tests_util::is_valid(&schema, &json!(null));
        tests_util::is_valid(&schema, &json!(["a"]));
    }

    #[test]
    fn fused_counts_code_points() {
        // 5 code points in 6 bytes; counting bytes would put it over
        tests_util::is_valid(&json!({"minLength": 5, "maxLength": 5}), &json!("héllo"));
        // 2 code points in 8 bytes
        tests_util::is_valid(&json!({"minLength": 2, "maxLength": 2}), &json!("💩💩"));
        tests_util::is_not_valid(&json!({"minLength": 3, "maxLength": 4}), &json!("💩💩"));
    }

    #[test]
    fn unsatisfiable_range_reports_both() {
        // Not `assert_locations`: its `zip` would pass on a single error.
        let validator = crate::validator_for(&json!({"minLength": 5, "maxLength": 3})).unwrap();
        let mut locations: Vec<String> = validator
            .iter_errors(&json!("abcd"))
            .map(|error| error.schema_path().as_str().to_string())
            .collect();
        locations.sort();
        assert_eq!(locations, ["/maxLength", "/minLength"]);
    }

    #[test]
    fn invalid_sibling_still_reported() {
        // An unusable bound keeps its own error instead of being folded away.
        assert!(crate::validator_for(&json!({"minLength": 1, "maxLength": -1})).is_err());
        assert!(crate::validator_for(&json!({"minLength": -1, "maxLength": 1})).is_err());
    }

    #[test]
    fn fused_absolute_keyword_locations() {
        tests_util::assert_absolute_keyword_locations(
            &json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$id": "https://example.com/s.json",
                "minLength": 20,
                "maxLength": 5
            }),
            &json!("secretvalue"),
            &[
                ("minLength", "https://example.com/s.json#/minLength"),
                ("maxLength", "https://example.com/s.json#/maxLength"),
            ],
        );
    }

    #[test]
    fn fused_evaluate_keyword_locations() {
        let validator = crate::validator_for(&json!({"minLength": 20, "maxLength": 5}))
            .expect("Invalid schema");
        let instance = json!("secretvalue");
        tests_util::assert_keyword_location(&validator, &instance, "", "/minLength");
        tests_util::assert_keyword_location(&validator, &instance, "", "/maxLength");
    }

    #[test]
    fn fused_error_locations() {
        let schema = json!({"minLength": 2, "maxLength": 3});
        tests_util::assert_error_locations(
            &schema,
            &json!("a"),
            &[("\"a\" is shorter than 2 characters", "", "/minLength")],
        );
        tests_util::assert_error_locations(
            &schema,
            &json!("abcd"),
            &[("\"abcd\" is longer than 3 characters", "", "/maxLength")],
        );
        tests_util::assert_error_locations(
            &json!({"minLength": 20, "maxLength": 5}),
            &json!("secretvalue"),
            &[
                (
                    "\"secretvalue\" is shorter than 20 characters",
                    "",
                    "/minLength",
                ),
                (
                    "\"secretvalue\" is longer than 5 characters",
                    "",
                    "/maxLength",
                ),
            ],
        );
    }
}
