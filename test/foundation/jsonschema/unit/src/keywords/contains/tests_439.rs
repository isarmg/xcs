#[cfg(test)]
mod tests {
    use crate::tests_util;
    use json_codec::{json, Value};
    use test_case::test_case;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"contains": {"const": 2}}),
            &json!([]),
            "/contains",
        );
    }

    #[test]
    fn subschema_does_not_replace_a_sibling_keyword() {
        let schema = json!({
            "items": {"type": "object"},
            "contains": {"items": {"type": "null"}},
            "minContains": 0,
            "maxContains": 3
        });
        tests_util::is_valid(&schema, &json!([{}]));
        tests_util::is_not_valid(&schema, &json!([null]));
    }

    // `minContains` and `maxContains` are inert, so `contains` asks for one match.
    #[test_case(&json!({}); "no bounds")]
    #[test_case(&json!({"minContains": 2}); "min")]
    #[test_case(&json!({"maxContains": 1}); "max")]
    #[test_case(&json!({"minContains": 2, "maxContains": 1}); "min and max")]
    #[test_case(&json!({"minContains": 0}); "min zero")]
    fn contains_bounds_without_validation_vocabulary(bounds: &Value) {
        let mut schema = json!({"contains": {"items": false}});
        schema
            .as_object_mut()
            .expect("object schema")
            .extend(bounds.as_object().expect("object bounds").clone());
        let instances = [json!([]), json!([[]]), json!([[], []]), json!([[1], []])];
        assert_eq!(
            tests_util::outcomes_with_only_vocabulary("applicator", &schema, &instances),
            [
                (
                    false,
                    vec![(
                        "/contains".to_string(),
                        "None of [] are valid under the given schema".to_string()
                    )]
                ),
                (true, Vec::new()),
                (true, Vec::new()),
                (true, Vec::new()),
            ]
        );
    }

    #[test_case(&json!({}), &json!([2]), "/contains"; "contains")]
    #[test_case(&json!({"minContains": 2}), &json!([1]), "/minContains"; "minContains")]
    #[test_case(&json!({"maxContains": 1}), &json!([1, 1]), "/maxContains"; "maxContains")]
    #[test_case(&json!({"maxContains": 1}), &json!([2]), "/maxContains"; "maxContains without a match")]
    #[test_case(&json!({"minContains": 2, "maxContains": 3}), &json!([1]), "/minContains"; "min of both")]
    #[test_case(
        &json!({"minContains": 2, "maxContains": 3}),
        &json!([1, 1, 1, 1]),
        "/maxContains";
        "max of both"
    )]
    fn error_locations(bounds: &Value, instance: &Value, schema_path: &str) {
        let mut schema = bounds.clone();
        schema["contains"] = json!({"const": 1});
        let message = format!("None of {instance} are valid under the given schema");
        tests_util::assert_error_locations(&schema, instance, &[(&message, "", schema_path)]);
    }

    #[test]
    fn malformed() {
        tests_util::assert_compile_error(
            &json!({"contains": 5}),
            "5 is not of types \"boolean\", \"object\"",
            "/contains",
        );
    }
}
