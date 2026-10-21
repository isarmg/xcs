#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use serde_json::{json, Value};
    use test_case::test_case;

    use super::{
        super::{Array, Json, JsonNumber, Node, Object},
        SerdeJson,
    };
    use crate::types::JsonType;

    // Generic on purpose: inherent `Value` methods shadow the trait on concrete `&Value`, and keyword code
    // only ever sees `F::Node<'_>`.
    fn assert_document_accessors<F: Json>(node: &F::Node<'_>) {
        let object = node.as_object().expect("object");

        let member = |name: &str| object.get(&F::prepare_key(name)).expect("present");

        assert!(member("object").as_object().is_some());
        assert_eq!(member("array").as_array().expect("array").len(), 3);
        assert_eq!(member("string").as_string().as_deref(), Some("héllo"));
        assert_eq!(member("string").json_type(), JsonType::String);
        assert_eq!(member("string").string_length(), Some(5));
        assert_eq!(
            member("integer").as_number().expect("number").as_u64(),
            Some(42)
        );
        assert_eq!(
            member("float").as_number().expect("number").as_f64(),
            Some(1.5)
        );
        assert_eq!(member("boolean").as_boolean(), Some(true));
        assert!(member("null").is_null());
        assert_eq!(member("null").json_type(), JsonType::Null);
        assert!(object.get(&F::prepare_key("missing")).is_none());
    }

    #[test]
    fn accessors_match_value_kinds() {
        let document = json!({
            "object": {"a": 1},
            "array": [1, 2, 3],
            "string": "héllo",
            "integer": 42,
            "float": 1.5,
            "boolean": true,
            "null": null
        });
        assert_document_accessors::<SerdeJson>(&&document);
    }

    #[test_case(&json!(1), &json!(1.0), true; "integer equals float")]
    #[test_case(&json!(1.0), &json!(1), true; "float equals integer")]
    #[test_case(&json!(1), &json!(2), false; "different integers")]
    #[test_case(&json!(true), &json!(1), false; "boolean is not a number")]
    #[test_case(&json!({"a": [1, {"b": 1.0}]}), &json!({"a": [1.0, {"b": 1}]}), true; "nested numeric equality")]
    #[test_case(&json!({"a": [1, {"b": 1.0}]}), &json!({"a": [1, {"b": 2}]}), false; "nested mismatch")]
    fn equals_value_follows_json_schema_semantics(left: &Value, right: &Value, expected: bool) {
        assert_eq!(left.equals_value(right), expected);
    }

    #[test_case("", 0; "empty")]
    #[test_case("héllo", 5; "multi-byte")]
    #[test_case("🦀🦀", 2; "astral plane")]
    fn string_length_counts_code_points(input: &str, expected: u64) {
        let value = json!(input);
        assert_eq!((&value).string_length(), Some(expected));
    }

    #[test]
    fn to_value_borrows() {
        let document = json!({"a": 1});
        let node = &document;
        assert!(matches!(node.to_value(), Cow::Borrowed(_)));
    }

    fn assert_identity_stability<F: Json>(node: &F::Node<'_>) {
        let child = node
            .as_object()
            .expect("object")
            .get(&F::prepare_key("a"))
            .expect("present");
        assert_eq!(node.identity(), node.identity());
        assert_ne!(node.identity(), child.identity());
    }

    #[test]
    fn identity_is_stable_per_node() {
        let document = json!({"a": {"b": 1}});
        assert_identity_stability::<SerdeJson>(&&document);
    }

    fn assert_iteration_order<F: Json>(node: &F::Node<'_>) {
        let object = node.as_object().expect("object");
        let names: Vec<_> = object
            .members()
            .map(|(name, _)| name.as_ref().to_owned())
            .collect();
        assert_eq!(names, ["a", "b"]);

        let items = object
            .get(&F::prepare_key("b"))
            .expect("present")
            .as_array()
            .expect("array");
        let collected: Vec<Option<u64>> = items
            .elements()
            .map(|item| item.as_number().and_then(|number| number.as_u64()))
            .collect();
        assert_eq!(collected, [Some(10), Some(20)]);
    }

    #[test]
    fn members_and_items_iterate_in_order() {
        let document = json!({"a": 1, "b": [10, 20]});
        assert_iteration_order::<SerdeJson>(&&document);
    }
}
