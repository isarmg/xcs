#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use test_case::test_case;

    use super::*;

    /// The parse output for `schema`, which is what the pass actually consumes.
    fn parsed_of(schema: &Value) -> ParseOutput {
        let registry = referencing::Registry::new()
            .add(
                "json-schema:///",
                referencing::Draft::Draft202012.create_resource_ref(schema),
            )
            .expect("the fixture is a resource")
            .draft(referencing::Draft::Draft202012)
            .prepare()
            .expect("the registry prepares");
        let resolver = registry
            .resolver(referencing::uri::from_str("json-schema:///").expect("a valid base URI"));
        let ctx = CanonicalizationContext::new(
            referencing::Draft::Draft202012,
            crate::options::PatternEngineOptions::default(),
            false,
        );
        parse::parse(schema, &ctx, &resolver, None)
            .expect("the fixture parses")
            .0
            .expect("the fixture is modeled")
    }

    /// The position of `schema`'s single `#` self-reference.
    fn self_reference_position(schema: &Value) -> Position {
        let parsed = parsed_of(schema);
        let mut found = Vec::new();
        collect_classified_references(&parsed.root, Position::InPlace, &mut found);
        let positions: Vec<Position> = found
            .into_iter()
            .filter(|(uri, _)| uri.as_ref() == "#")
            .map(|(_, position)| position)
            .collect();
        assert_eq!(
            positions.len(),
            1,
            "the fixture holds exactly one self-reference, found {positions:?}"
        );
        positions[0]
    }

    #[test_case(&json!({"properties": {"a": {"$ref": "#"}}}), Position::Consuming ; "properties")]
    #[test_case(&json!({"patternProperties": {"^a": {"$ref": "#"}}}), Position::Consuming ; "pattern_properties")]
    #[test_case(&json!({"type": "object", "additionalProperties": {"$ref": "#"}}), Position::Consuming ; "additional_properties")]
    #[test_case(&json!({"type": "object", "propertyNames": {"$ref": "#"}}), Position::Consuming ; "property_names")]
    #[test_case(&json!({"type": "array", "items": {"$ref": "#"}}), Position::Consuming ; "items")]
    #[test_case(&json!({"type": "array", "prefixItems": [{"$ref": "#"}]}), Position::Consuming ; "prefix_items")]
    #[test_case(&json!({"contains": {"$ref": "#"}}), Position::Consuming ; "contains")]
    #[test_case(&json!({"allOf": [{"$ref": "#"}, {"type": "integer"}]}), Position::InPlace ; "all_of")]
    #[test_case(&json!({"anyOf": [{"$ref": "#"}, {"type": "integer"}]}), Position::InPlace ; "any_of")]
    #[test_case(&json!({"not": {"$ref": "#"}}), Position::InPlace ; "not")]
    fn classifies_reference_position(schema: &Value, expected: Position) {
        assert_eq!(self_reference_position(schema), expected);
    }

    #[test]
    fn nesting_below_a_consuming_position_stays_consuming() {
        let schema = json!({"properties": {"a": {"anyOf": [{"$ref": "#"}, {"type": "null"}]}}});
        assert_eq!(self_reference_position(&schema), Position::Consuming);
    }

    /// The graph the pass sees, built from the parse rather than the emitted form - so a fixture
    /// does not have to be one the pass leaves alone.
    fn edges_of(schema: &Value) -> ReferenceEdges {
        let parsed = parsed_of(schema);
        reference_edges(&parsed.root, &parsed.definitions)
    }

    fn guarded(schema: &Value) -> Vec<String> {
        let mut names: Vec<String> = guarded_members(&edges_of(schema))
            .iter()
            .map(ToString::to_string)
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_consuming_self_cycle_qualifies() {
        let schema = json!({"type": "object", "properties": {"a": {"$ref": "#"}}});
        assert_eq!(guarded(&schema), vec!["#".to_string()]);
    }

    /// The cycle the pass actually decides. Reachable only because the graph is built from the
    /// parse: the emitted form of this document is `false`, with no graph left to inspect.
    #[test]
    fn a_required_consuming_self_cycle_qualifies() {
        let schema =
            json!({"type": "object", "required": ["a"], "properties": {"a": {"$ref": "#"}}});
        assert_eq!(guarded(&schema), vec!["#".to_string()]);
    }

    #[test]
    fn an_in_place_self_cycle_is_excluded() {
        let schema = json!({"$ref": "#/$defs/a", "$defs": {"a": {"allOf": [{"$ref": "#/$defs/a"}, {"type": "integer"}]}}});
        assert_eq!(guarded(&schema), Vec::<String>::new());
    }

    /// Every cycle here passes through the consuming edge.
    #[test]
    fn a_mixed_cycle_qualifies() {
        let schema = json!({
            "$ref": "#/$defs/a",
            "$defs": {
                "a": {"allOf": [{"$ref": "#/$defs/b"}, {"type": "object"}]},
                "b": {"type": "object", "properties": {"x": {"$ref": "#/$defs/a"}}}
            }
        });
        assert_eq!(
            guarded(&schema),
            vec!["#/$defs/a".to_string(), "#/$defs/b".to_string()]
        );
    }

    /// A member keeping a consuming cycle of its own stays guarded when an ill-founded member
    /// shares its component; one whose only cycle ran through the excluded member does not.
    #[test]
    fn only_the_ill_founded_member_is_excluded() {
        let schema = json!({
            "$ref": "#/$defs/a",
            "$defs": {
                "a": {"type": "object", "properties": {
                    "x": {"$ref": "#/$defs/a"},
                    "y": {"$ref": "#/$defs/b"}
                }},
                "b": {"anyOf": [{"$ref": "#/$defs/b"}, {"$ref": "#/$defs/a"}]}
            }
        });
        assert_eq!(guarded(&schema), vec!["#/$defs/a".to_string()]);
    }

    #[test]
    fn an_acyclic_document_has_no_members() {
        let schema = json!({"$ref": "#/$defs/a", "$defs": {"a": {"type": "integer"}}});
        assert_eq!(guarded(&schema), Vec::<String>::new());
    }
}
