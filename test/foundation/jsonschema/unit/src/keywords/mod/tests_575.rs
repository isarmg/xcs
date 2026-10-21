#[cfg(test)]
mod tests {
    use json_codec::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"prefixItems": [{}], "items": {"type": "integer"}}), &json!([ null, 2, 3, "foo" ]), r#""foo" is not of type "integer""#)]
    #[test_case(&json!({"prefixItems": [{}, {}, {}], "items": false}), &json!([ 1, 2, 3, 4 ]), r"False schema does not allow 4")]
    #[test_case(&json!({"prefixItems": [{}, {}, {}], "items": false}), &json!([ 1, 2, 3, 4, 5 ]), r"False schema does not allow 4")]
    #[test_case(&json!({"properties": {"foo": {}, "bar": {}}, "patternProperties": { "^v": {} }, "additionalProperties": false}), &json!({"foo" : 1, "bar" : 2, "quux" : "boom"}), r"Additional properties are not allowed ('quux' was unexpected)")]
    #[test_case(&json!({"anyOf": [{"type": "integer"}, {"minimum": 2}]}), &json!(1.5), r"1.5 is not valid under any of the schemas listed in the 'anyOf' keyword")]
    #[test_case(&json!({"const": 2}), &json!(5), r"2 was expected")]
    #[test_case(&json!({"contains": {"minimum": 5}}), &json!([2, 3, 4]), r"None of [2,3,4] are valid under the given schema")]
    #[test_case(&json!({"enum": [1]}), &json!(4), r"4 is not one of 1")]
    #[test_case(&json!({"enum": [1, 2]}), &json!(4), r"4 is not one of 1 or 2")]
    #[test_case(&json!({"enum": [1, 2, 3]}), &json!(4), r"4 is not one of 1, 2 or 3")]
    #[test_case(&json!({"enum": [1, 2, 3, 4]}), &json!(5), r"5 is not one of 1, 2 or 2 other candidates")]
    #[test_case(&json!({"enum": [1, 2, 3, 4, 5]}), &json!(6), r"6 is not one of 1, 2 or 3 other candidates")]
    #[test_case(&json!({"exclusiveMaximum": 3}), &json!(3.0), r"3.0 is greater than or equal to the maximum of 3")]
    #[test_case(&json!({"exclusiveMaximum": 3.0}), &json!(3.0), r"3.0 is greater than or equal to the maximum of 3.0")]
    #[test_case(&json!({"exclusiveMinimum": 1}), &json!(1.0), r"1.0 is less than or equal to the minimum of 1")]
    #[test_case(&json!({"exclusiveMinimum": 1.0}), &json!(1), r"1 is less than or equal to the minimum of 1.0")]
    #[test_case(&json!({"format": "ipv4"}), &json!("2001:0db8:85a3:0000:0000:8a2e:0370:7334"), r#""2001:0db8:85a3:0000:0000:8a2e:0370:7334" is not a "ipv4""#)]
    #[test_case(&json!({"maximum": 3}), &json!(3.5), r"3.5 is greater than the maximum of 3")]
    #[test_case(&json!({"maximum": 3.0}), &json!(3.5), r"3.5 is greater than the maximum of 3.0")]
    #[test_case(&json!({"minimum": 3}), &json!(2.5), r"2.5 is less than the minimum of 3")]
    #[test_case(&json!({"minimum": 3.0}), &json!(2.5), r"2.5 is less than the minimum of 3.0")]
    #[test_case(&json!({"maxItems": 2}), &json!([1, 2, 3]), r"[1,2,3] has more than 2 items")]
    #[test_case(&json!({"maxLength": 2}), &json!("foo"), r#""foo" is longer than 2 characters"#)]
    #[test_case(&json!({"maxProperties": 2}), &json!({"bar": 2, "baz": 3, "foo": 1}), r#"{"bar":2,"baz":3,"foo":1} has more than 2 properties"#)]
    #[test_case(&json!({"minimum": 1.1}), &json!(0.6), r"0.6 is less than the minimum of 1.1")]
    #[test_case(&json!({"minItems": 1}), &json!([]), r"[] has less than 1 item")]
    #[test_case(&json!({"minLength": 2}), &json!("f"), r#""f" is shorter than 2 characters"#)]
    #[test_case(&json!({"minProperties": 1}), &json!({}), r"{} has less than 1 property")]
    #[test_case(&json!({"multipleOf": 2}), &json!(7), r"7 is not a multiple of 2")]
    #[test_case(&json!({"not": {"type": "integer"}}), &json!(1), r#"{"type":"integer"} is not allowed for 1"#)]
    #[test_case(&json!({"oneOf": [{"type": "integer"}, {"minimum": 2}]}), &json!(1.1), r"1.1 is not valid under any of the schemas listed in the 'oneOf' keyword")]
    #[test_case(&json!({"oneOf": [{"type": "integer"}, {"minimum": 2}]}), &json!(3), r"3 is valid under more than one of the schemas listed in the 'oneOf' keyword")]
    #[test_case(&json!({"pattern": "^a*$"}), &json!("abc"), r#""abc" does not match "^a*$""#)]
    #[test_case(&json!({"properties": {"foo": {}, "bar": {}}, "required": ["foo"]}), &json!({"bar": 1}), r#""foo" is a required property"#)]
    #[test_case(&json!({"type": "integer"}), &json!(1.1), r#"1.1 is not of type "integer""#)]
    #[test_case(&json!({"type": ["integer", "string"]}), &json!(null), r#"null is not of types "integer", "string""#)]
    #[test_case(&json!({"uniqueItems": true}), &json!([1, 1]), r"[1,1] has non-unique elements")]
    fn error_message(schema: &Value, instance: &Value, expected: &str) {
        let validator = crate::options()
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema");
        let errors: Vec<_> = validator.iter_errors(instance).collect();
        assert_eq!(errors[0].to_string(), expected);
    }

    // Extra cases not covered by JSON test suite
    #[test_case(&json!({"additionalProperties": {"type": "string"}}))]
    #[test_case(&json!({"additionalProperties": {"type": "string"}, "properties": {"foo": {}}}))]
    #[test_case(&json!({"additionalProperties": {"type": "string"}, "patternProperties": {"f.*o": {"type": "integer"}}}))]
    #[test_case(&json!({"additionalProperties": {"type": "string"}, "properties": {"foo": {}}, "patternProperties": {"f.*o": {"type": "integer"}}}))]
    #[test_case(&json!({"additionalProperties": false}))]
    #[test_case(&json!({"additionalProperties": false, "properties": {"foo": {}}}))]
    #[test_case(&json!({"additionalProperties": false, "patternProperties": {"f.*o": {"type": "integer"}}}))]
    #[test_case(&json!({"additionalProperties": false, "properties": {"foo": {}}, "patternProperties": {"f.*o": {"type": "integer"}}}))]
    #[test_case(&json!({"additionalItems": false, "prefixItems": [{"type": "string"}]}))]
    #[test_case(&json!({"additionalItems": {"type": "integer"}, "prefixItems": [{"type": "string"}]}))]
    #[test_case(&json!({"contains": {"minimum": 5}}))]
    #[test_case(&json!({"contentMediaType": "application/json"}))]
    #[test_case(&json!({"contentEncoding": "base64"}))]
    #[test_case(&json!({"contentEncoding": "base64", "contentMediaType": "application/json"}))]
    #[test_case(&json!({"dependencies": {"bar": ["foo"]}}))]
    #[test_case(&json!({"exclusiveMaximum": 5}))]
    #[test_case(&json!({"exclusiveMinimum": 5}))]
    #[test_case(&json!({"format": "ipv4"}))]
    #[test_case(&json!({"maximum": 2}))]
    #[test_case(&json!({"maxItems": 2}))]
    #[test_case(&json!({"maxProperties": 2}))]
    #[test_case(&json!({"minProperties": 2}))]
    #[test_case(&json!({"multipleOf": 2.5}))]
    #[test_case(&json!({"multipleOf": 2}))]
    #[test_case(&json!({"required": ["a"]}))]
    #[test_case(&json!({"pattern": "^a"}))]
    #[test_case(&json!({"patternProperties": {"f.*o": {"type": "integer"}}}))]
    #[test_case(&json!({"propertyNames": {"maxLength": 3}}))]
    fn is_valid_another_type(schema: &Value) {
        let instance = json!(null);
        assert!(crate::options()
            .should_validate_formats(true)
            .build(schema)
            .expect("Invalid schema")
            .is_valid(&instance));
    }
    #[test_case(&json!({"additionalProperties": false}), &json!({}))]
    #[test_case(&json!({"additionalItems": false, "items": true}), &json!([]))]
    fn is_valid(schema: &Value, instance: &Value) {
        assert!(crate::is_valid(schema, instance));
    }

    #[test_case(&json!({"type": "number"}), &json!(42))]
    #[test_case(&json!({"type": ["number", "null"]}), &json!(42))]
    fn integer_is_valid_number_multi_type(schema: &Value, instance: &Value) {
        // See: GH-147
        assert!(crate::is_valid(schema, instance));
    }
    // enum: Number
    #[test_case(&json!({"enum": [0.0]}), &json!(0))]
    // enum: Array
    #[test_case(&json!({"enum": [[1.0]]}), &json!([1]))]
    // enum: Object
    #[test_case(&json!({"enum": [{"a": 1.0}]}), &json!({"a": 1}))]
    // enum:: Object in Array
    #[test_case(&json!({"enum": [[{"b": 1.0}]]}), &json!([{"b": 1}]))]
    // enum:: Array in Object
    #[test_case(&json!({"enum": [{"c": [1.0]}]}), &json!({"c": [1]}))]
    // const: Number
    #[test_case(&json!({"const": 0.0}), &json!(0))]
    // const: Array
    #[test_case(&json!({"const": [1.0]}), &json!([1]))]
    // const: Object
    #[test_case(&json!({"const": {"a": 1.0}}), &json!({"a": 1}))]
    // const:: Object in Array
    #[test_case(&json!({"const": [{"b": 1.0}]}), &json!([{"b": 1}]))]
    // const:: Array in Object
    #[test_case(&json!({"const": {"c": [1.0]}}), &json!({"c": [1]}))]
    fn numeric_equivalence(schema: &Value, instance: &Value) {
        // See: GH-149
        assert!(crate::is_valid(schema, instance));
    }

    #[test_case(crate::Draft::Draft4)]
    #[test_case(crate::Draft::Draft6)]
    #[test_case(crate::Draft::Draft7)]
    #[test_case(crate::Draft::Draft201909)]
    #[test_case(crate::Draft::Draft202012)]
    fn size_bounds_past_u64(draft: crate::Draft) {
        // No collection can reach `2^64`, so a `max*` bound there accepts every instance and a
        // `min*` bound rejects every one - but every draft must build the schema.
        const LIMIT: &str = "18446744073709551616";
        for (keyword, instance) in [
            ("maxLength", json!("ab")),
            ("minLength", json!("ab")),
            ("maxItems", json!([1, 2])),
            ("minItems", json!([1, 2])),
            ("maxProperties", json!({"a": 1})),
            ("minProperties", json!({"a": 1})),
        ] {
            let mut cases = vec![
                (
                    format!(r#"{{"{keyword}": {LIMIT}}}"#),
                    keyword.starts_with("max"),
                ),
                // The bound sits under `properties/a`, which none of these instances reach.
                (
                    format!(r#"{{"properties": {{"a": {{"{keyword}": {LIMIT}}}}}}}"#),
                    true,
                ),
            ];
            if keyword.ends_with("Items") {
                // The fused array-shape validator reads the bound on its own.
                cases.push((
                    format!(r#"{{"type": "array", "items": {{}}, "{keyword}": {LIMIT}}}"#),
                    keyword.starts_with("max"),
                ));
            }
            for (schema, expected) in cases {
                let schema: Value = json_codec::from_str(&schema).expect("Invalid JSON");
                let validator = crate::options()
                    .with_draft(draft)
                    .build(&schema)
                    .unwrap_or_else(|error| panic!("{schema} does not build: {error}"));
                assert_eq!(
                    validator.is_valid(&instance),
                    expected,
                    "{schema} against {instance}"
                );
            }
        }
    }

    #[test_case("-1")]
    #[test_case("1.5")]
    #[test_case(r#""x""#)]
    // `18446744073709551616.0` is indistinguishable from the integer without the raw JSON text.
    #[cfg_attr(feature = "arbitrary-precision", test_case("18446744073709551616.0"))]
    fn invalid_size_bounds_still_rejected(limit: &str) {
        for keyword in ["maxLength", "minItems", "maxProperties"] {
            let schema: Value = json_codec::from_str(&format!(r#"{{"{keyword}": {limit}}}"#))
                .expect("Invalid JSON");
            assert!(
                crate::options()
                    .with_draft(crate::Draft::Draft4)
                    .build(&schema)
                    .is_err(),
                "{schema} should not build"
            );
        }
    }

    #[test]
    fn required_all_properties() {
        // See: GH-190
        let schema = json!({"required": ["foo", "bar"]});
        let instance = json!({});
        let validator = crate::validator_for(&schema).unwrap();
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].to_string(), r#""foo" is a required property"#);
        assert_eq!(errors[1].to_string(), r#""bar" is a required property"#);
    }
}
