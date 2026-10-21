#[cfg(test)]
mod tests {
    use referencing::Draft;
    use json_codec::{json, Value};

    use super::*;
    use crate::{canonical::ir::BoundRational, options::PatternEngineOptions};

    fn context() -> CanonicalizationContext {
        CanonicalizationContext::new(Draft::Draft202012, PatternEngineOptions::default(), false)
    }

    const TYPES: [JsonType; 7] = [
        JsonType::Null,
        JsonType::Boolean,
        JsonType::Integer,
        JsonType::Number,
        JsonType::String,
        JsonType::Array,
        JsonType::Object,
    ];

    // One value per equivalence class of the type vocabulary; `1` and `1.5` are distinct classes
    // because an integer satisfies both `integer` and `number` while a fraction satisfies only
    // `number`.
    fn representatives() -> [Value; 7] {
        [
            json!(null),
            json!(true),
            json!(1),
            json!(1.5),
            json!("x"),
            json!([]),
            json!({}),
        ]
    }

    fn admits(set: JsonTypeSet, value: &Value) -> bool {
        match value {
            Value::Null => set.contains(JsonType::Null),
            Value::Bool(_) => set.contains(JsonType::Boolean),
            Value::Number(number) if number.is_i64() => {
                set.contains(JsonType::Integer) || set.contains(JsonType::Number)
            }
            Value::Number(_) => set.contains(JsonType::Number),
            Value::String(_) => set.contains(JsonType::String),
            Value::Array(_) => set.contains(JsonType::Array),
            Value::Object(_) => set.contains(JsonType::Object),
        }
    }

    // Membership for the canonical shapes a negation can take: a type set, its boolean-schema
    // collapses, the value-set forms of a lone `null` or `boolean` type, and the
    // non-integer-number leaf beside its union.
    #[allow(clippy::wildcard_enum_match_arm)]
    fn negation_admits(schema: &Schema, value: &Value) -> bool {
        match schema.kind() {
            SchemaKind::True => true,
            SchemaKind::False => false,
            SchemaKind::MultiType(set) => admits(*set, value),
            SchemaKind::Const(constant) => {
                assert_eq!(constant.as_value(), &Value::Null);
                value.is_null()
            }
            SchemaKind::Enum(values) => {
                let members: Vec<&Value> = values
                    .as_slice()
                    .iter()
                    .map(CanonicalJson::as_value)
                    .collect();
                assert_eq!(members, [&Value::Bool(false), &Value::Bool(true)]);
                value.is_boolean()
            }
            SchemaKind::AnyOf(branches) => branches
                .as_slice()
                .iter()
                .any(|branch| negation_admits(branch, value)),
            SchemaKind::Number(leaf) => {
                assert!(leaf.get().minimum.is_none());
                assert!(leaf.get().maximum.is_none());
                assert!(leaf.get().multiple_of.is_empty());
                let barred: Vec<Number> = leaf
                    .get()
                    .not_multiple_of
                    .as_slice()
                    .iter()
                    .map(BoundRational::to_number)
                    .collect();
                assert_eq!(barred, [Number::from(1)]);
                matches!(value, Value::Number(number) if !number.is_i64() && !number.is_u64())
            }
            other => {
                panic!("scaffold negation of a type set is a type-set shape, got {other:?}")
            }
        }
    }

    // The scaffold's domain is finite, so the negation-membership law is proven exhaustively: for
    // every one of the 128 type sets, the negation admits a value exactly when the original does
    // not.
    #[test]
    fn type_set_negation_partitions_the_value_space() {
        let ctx = context();
        for mask in 0u8..128 {
            let mut set = JsonTypeSet::empty();
            for ty in TYPES {
                if mask & ty as u8 != 0 {
                    set = set.insert(ty);
                }
            }
            let schema = Schema::new(SchemaKind::MultiType(set));
            let negation = negate_in_place(&schema, &DefinitionMap::new(), &ctx)
                .expect("expressible negation");
            for value in &representatives() {
                assert_ne!(
                    admits(set, value),
                    negation_admits(&negation, value),
                    "membership not partitioned for set {set:?} on {value}"
                );
            }
        }
    }

    #[test]
    fn boolean_schemas_negate_to_each_other() {
        let ctx = context();
        assert!(matches!(
            negate_in_place(&Schema::truthy(), &DefinitionMap::new(), &ctx)
                .map(|s| s.kind().clone()),
            Some(SchemaKind::False)
        ));
        assert!(matches!(
            negate_in_place(&Schema::falsy(), &DefinitionMap::new(), &ctx)
                .map(|s| s.kind().clone()),
            Some(SchemaKind::True)
        ));
    }
}
