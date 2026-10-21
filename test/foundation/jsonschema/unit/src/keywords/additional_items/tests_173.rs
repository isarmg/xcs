#[cfg(test)]
mod tests {
    use crate::tests_util;
    use referencing::Draft;
    use serde_json::{json, Value};
    use test_case::test_case;

    fn validator_with(draft: Draft, schema: &Value) -> crate::Validator {
        // Draft 4's metaschema forbids boolean schemas, so meta-validation is off to let one
        // builder cover every draft.
        crate::options()
            .with_draft(draft)
            .without_schema_validation()
            .build(schema)
            .expect("schema compiles")
    }

    // A boolean `items` leaves no tuple tail, so `additionalItems` is ignored and only `items`
    // can reject. The keyword constrains array elements, so it must never touch a non-array.
    // Dispatch is draft-agnostic: 2020-12 goes through the same `Bool` arm as older drafts.
    #[test_case(Draft::Draft4, &json!({"additionalItems": false, "items": false}), &json!(null); "draft4 null")]
    #[test_case(Draft::Draft4, &json!({"additionalItems": false, "items": false}), &json!([]); "draft4 empty array")]
    #[test_case(Draft::Draft4, &json!({"additionalItems": false, "items": false}), &json!("a"); "draft4 string")]
    #[test_case(Draft::Draft4, &json!({"additionalItems": false, "items": true}), &json!(null); "draft4 items true")]
    #[test_case(Draft::Draft6, &json!({"additionalItems": false, "items": false}), &json!(null); "draft6 null")]
    #[test_case(Draft::Draft6, &json!({"additionalItems": false, "items": false}), &json!([]); "draft6 empty array")]
    #[test_case(Draft::Draft6, &json!({"additionalItems": false, "items": false}), &json!("a"); "draft6 string")]
    #[test_case(Draft::Draft6, &json!({"additionalItems": false, "items": true}), &json!(null); "draft6 items true")]
    #[test_case(Draft::Draft7, &json!({"additionalItems": false, "items": false}), &json!(null); "draft7 null")]
    #[test_case(Draft::Draft7, &json!({"additionalItems": false, "items": false}), &json!([]); "draft7 empty array")]
    #[test_case(Draft::Draft7, &json!({"additionalItems": false, "items": false}), &json!("a"); "draft7 string")]
    #[test_case(Draft::Draft7, &json!({"additionalItems": false, "items": true}), &json!(null); "draft7 items true")]
    #[test_case(Draft::Draft201909, &json!({"additionalItems": false, "items": false}), &json!(null); "draft2019 null")]
    #[test_case(Draft::Draft201909, &json!({"additionalItems": false, "items": false}), &json!([]); "draft2019 empty array")]
    #[test_case(Draft::Draft201909, &json!({"additionalItems": false, "items": false}), &json!("a"); "draft2019 string")]
    #[test_case(Draft::Draft201909, &json!({"additionalItems": false, "items": true}), &json!(null); "draft2019 items true")]
    #[test_case(Draft::Draft202012, &json!({"additionalItems": false, "items": false}), &json!(null); "draft2020 null")]
    #[test_case(Draft::Draft202012, &json!({"additionalItems": false, "items": false}), &json!([]); "draft2020 empty array")]
    #[test_case(Draft::Draft202012, &json!({"additionalItems": false, "items": false}), &json!("a"); "draft2020 string")]
    #[test_case(Draft::Draft202012, &json!({"additionalItems": false, "items": true}), &json!(null); "draft2020 items true")]
    fn boolean_items_makes_additional_items_inert(draft: Draft, schema: &Value, instance: &Value) {
        let validator = validator_with(draft, schema);
        tests_util::is_valid_with(&validator, instance);
    }

    // A non-array `items` value leaves no tuple tail either, so `additionalItems` beside it is
    // ignored just like the invalid `items` itself. Only a registry resource can carry such a
    // pair, since top-level builds are meta-validated.
    #[test_case(&json!(42))]
    #[test_case(&json!("items"))]
    #[test_case(&json!(null))]
    fn non_array_items_makes_additional_items_inert(items: &Value) {
        let resource = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "additionalItems": {"type": "string"},
            "items": items,
        });
        let registry = crate::Registry::new()
            .add("https://example.com/tail", &resource)
            .expect("resource accepted")
            .prepare()
            .expect("registry build failed");
        let validator = crate::options()
            .with_registry(&registry)
            .build(&json!({"$ref": "https://example.com/tail"}))
            .expect("Invalid schema");
        assert!(validator.is_valid(&json!([1, 2])));
        assert!(validator.is_valid(&json!(null)));
    }

    // A non-schema `additionalItems` beside an array-form `items` fails the build, like
    // `additionalProperties` does. Only a registry resource can carry such a value, since
    // top-level builds are meta-validated.
    #[test_case(&json!("false"); "string")]
    #[test_case(&json!(5); "number")]
    #[test_case(&json!(null); "null")]
    #[test_case(&json!([false]); "array")]
    fn non_schema_additional_items_fails_compilation(value: &Value) {
        let resource = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "items": [{"type": "integer"}],
            "additionalItems": value,
        });
        let registry = crate::Registry::new()
            .add("https://example.com/tail", &resource)
            .expect("resource accepted")
            .prepare()
            .expect("registry build failed");
        let error = crate::options()
            .with_registry(&registry)
            .build(&json!({"$ref": "https://example.com/tail"}))
            .expect_err("Should fail to build");
        assert_eq!(
            error.to_string(),
            format!("{value} is not of types \"boolean\", \"object\"")
        );
    }

    // `items: false` still forbids every element, with or without `additionalItems` beside it.
    #[test_case(&json!({"additionalItems": false, "items": false}))]
    #[test_case(&json!({"additionalItems": true, "items": false}))]
    #[test_case(&json!({"items": false}))]
    fn boolean_items_still_rejects_elements(schema: &Value) {
        let validator = validator_with(Draft::Draft7, schema);
        tests_util::is_not_valid_with(&validator, &json!([1]));
    }

    // Beside a boolean `items` no additionalItems validator exists, so `/items` is the only
    // possible schema path; with array-form `items` the tail error comes from `/additionalItems`.
    #[test_case(&json!({"additionalItems": false, "items": false}), &json!([1]), "/items")]
    #[test_case(&json!({"additionalItems": false, "items": [{}]}), &json!([1, 2]), "/additionalItems")]
    #[test_case(&json!({"additionalItems": {"type": "string"}, "items": [{}]}), &json!([1, 2]), "/additionalItems/type")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        let validator = crate::options()
            .with_draft(Draft::Draft7)
            .build(schema)
            .expect("Invalid schema");
        let error = validator.validate(instance).expect_err("Should fail");
        assert_eq!(error.schema_path().as_str(), expected);
    }
}
