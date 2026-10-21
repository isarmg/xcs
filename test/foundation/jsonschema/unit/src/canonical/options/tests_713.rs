#[cfg(test)]
mod tests {
    use super::{options, PreparedDocument};
    use serde_json::{json, Value};

    /// Every pointer in `document`, deepest first.
    fn every_pointer(document: &Value) -> Vec<String> {
        fn walk(value: &Value, pointer: &str, out: &mut Vec<String>) {
            match value {
                Value::Object(map) => {
                    for (key, child) in map {
                        walk(child, &format!("{pointer}/{key}"), out);
                    }
                }
                Value::Array(items) => {
                    for (index, child) in items.iter().enumerate() {
                        walk(child, &format!("{pointer}/{index}"), out);
                    }
                }
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
            }
            out.push(pointer.to_string());
        }
        let mut out = Vec::new();
        walk(document, "", &mut out);
        out
    }

    /// A prepared document holding no bodies yet, whose first read parses every definition itself.
    fn unseeded(document: &Value) -> PreparedDocument<'_> {
        options().prepare(document).expect("prepares")
    }

    fn documents() -> Vec<Value> {
        vec![
            json!({
                "$defs": {"Named": {"type": "object", "required": ["name"]}},
                "properties": {"a": {"$ref": "#/$defs/Named"}},
                "allOf": [{"$ref": "#/$defs/Named"}]
            }),
            // A body reached only through another body.
            json!({
                "$defs": {
                    "Inner": {"type": "integer", "minimum": 5},
                    "Outer": {"allOf": [{"$ref": "#/$defs/Inner"}, {"maximum": 3}]}
                },
                "properties": {"a": {"$ref": "#/$defs/Outer"}}
            }),
            // A cycle, which the definition fixpoint has to settle.
            json!({
                "$defs": {"Node": {
                    "type": "object",
                    "properties": {"next": {"$ref": "#/$defs/Node"}}
                }},
                "$ref": "#/$defs/Node"
            }),
            // A subresource carrying its own `$id`, so keys are generated against another base.
            json!({
                "$defs": {"Sub": {
                    "$id": "https://example.com/sub",
                    "$defs": {"Leaf": {"type": "string"}},
                    "properties": {"leaf": {"$ref": "#/$defs/Leaf"}}
                }},
                "properties": {"s": {"$ref": "https://example.com/sub"}}
            }),
            // A dynamic reference, where a key is specialized by the scope it was reached through.
            json!({
                "$defs": {"Items": {
                    "$dynamicAnchor": "T",
                    "type": "array",
                    "items": {"$dynamicRef": "#T"}
                }},
                "properties": {"a": {"$ref": "#/$defs/Items"}}
            }),
            // A definition the document's own root stops referencing once it folds.
            json!({
                "$defs": {"Dead": {"allOf": [{"type": "string"}, {"type": "integer"}]}},
                "properties": {"a": {"$ref": "#/$defs/Dead"}}
            }),
        ]
    }

    #[test]
    fn a_seeded_selection_reads_the_same_as_one_that_parses_its_own_definitions() {
        for document in documents() {
            let seeded = options().prepare(&document).expect("prepares");
            // Primed by reading the whole document, which leaves every body it reached behind.
            let _ = seeded.canonicalize();
            for pointer in every_pointer(&document) {
                let left = seeded.canonicalize_at(&pointer).map(|s| s.to_json_schema());
                // Prepared afresh, so this read holds no body and parses each one it reaches.
                let right = unseeded(&document)
                    .canonicalize_at(&pointer)
                    .map(|s| s.to_json_schema());
                assert_eq!(
                    left.as_ref().ok(),
                    right.as_ref().ok(),
                    "{pointer} of {document}"
                );
                assert_eq!(left.is_err(), right.is_err(), "{pointer} of {document}");
            }
        }
    }

    #[test]
    fn a_seeded_document_reads_the_same_as_one_that_parses_it_alone() {
        for document in documents() {
            let seeded = options().prepare(&document).expect("prepares");
            // Primed by reading every subschema, so the bodies the document read reuses were
            // parsed for another target.
            for pointer in every_pointer(&document) {
                let _ = seeded.canonicalize_at(&pointer);
            }
            assert_eq!(
                seeded
                    .canonicalize()
                    .expect("canonicalizes")
                    .to_json_schema(),
                unseeded(&document)
                    .canonicalize()
                    .expect("canonicalizes")
                    .to_json_schema(),
                "{document}"
            );
        }
    }

    #[test]
    fn reading_the_same_subschema_again_reads_the_same() {
        for document in documents() {
            let prepared = options().prepare(&document).expect("prepares");
            for pointer in every_pointer(&document) {
                let first = prepared
                    .canonicalize_at(&pointer)
                    .map(|s| s.to_json_schema());
                let again = prepared
                    .canonicalize_at(&pointer)
                    .map(|s| s.to_json_schema());
                assert_eq!(first.ok(), again.ok(), "{pointer} of {document}");
            }
        }
    }
}
