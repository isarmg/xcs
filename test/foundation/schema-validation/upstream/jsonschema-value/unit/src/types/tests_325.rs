#[cfg(test)]
mod tests {
    use super::*;
    use json_codec::json;
    use test_case::test_case;

    #[test]
    fn type_bitor_builds_set() {
        let set = JsonType::String | JsonType::Integer;
        assert!(set.contains(JsonType::String));
        assert!(set.contains(JsonType::Integer));
        assert!(!set.contains(JsonType::Null));
        let extended = set | JsonType::Null;
        assert_eq!(
            extended,
            JsonType::String | JsonType::Integer | JsonType::Null
        );
        assert!(extended.contains(JsonType::Null));
    }

    #[test_case("array" => Ok(JsonType::Array) ; "parse array")]
    #[test_case("boolean" => Ok(JsonType::Boolean) ; "parse boolean")]
    #[test_case("integer" => Ok(JsonType::Integer) ; "parse integer")]
    #[test_case("null" => Ok(JsonType::Null) ; "parse null")]
    #[test_case("number" => Ok(JsonType::Number) ; "parse number")]
    #[test_case("object" => Ok(JsonType::Object) ; "parse object")]
    #[test_case("string" => Ok(JsonType::String) ; "parse string")]
    #[test_case("invalid" => Err(()) ; "parse invalid")]
    fn test_from_str(input: &str) -> Result<JsonType, ()> {
        JsonType::from_str(input)
    }

    #[test_case(JsonType::Array => "array" ; "display array")]
    #[test_case(JsonType::Boolean => "boolean" ; "display boolean")]
    #[test_case(JsonType::Integer => "integer" ; "display integer")]
    #[test_case(JsonType::Null => "null" ; "display null")]
    #[test_case(JsonType::Number => "number" ; "display number")]
    #[test_case(JsonType::Object => "object" ; "display object")]
    #[test_case(JsonType::String => "string" ; "display string")]
    fn test_display(json_type: JsonType) -> String {
        json_type.to_string()
    }

    #[test_case(&json!(null) => JsonType::Null ; "value null")]
    #[test_case(&json!(true) => JsonType::Boolean ; "value boolean")]
    #[test_case(&json!(42) => JsonType::Number ; "value number int")]
    #[test_case(&json!(1.12) => JsonType::Number ; "value number float")]
    #[test_case(&json!("hello") => JsonType::String ; "value string")]
    #[test_case(&json!([1, 2, 3]) => JsonType::Array ; "value array")]
    #[test_case(&json!({"key": "value"}) => JsonType::Object ; "value object")]
    fn test_from_value(value: &Value) -> JsonType {
        JsonType::from(value)
    }

    #[test]
    fn test_insert_types() {
        let mut set = JsonTypeSet::empty();
        set = set.insert(JsonType::String);
        assert!(set.contains(JsonType::String));
        assert!(!set.contains(JsonType::Number));

        set = set.insert(JsonType::Number);
        assert!(set.contains(JsonType::String));
        assert!(set.contains(JsonType::Number));
        assert!(!set.contains(JsonType::Array));
    }

    #[test]
    fn test_from_json_type() {
        let set = JsonTypeSet::from(JsonType::String);
        assert!(set.contains(JsonType::String));
        assert_eq!(set.len(), 1);
    }

    #[cfg(feature = "json-codec")]
    #[test_case(&json!(null), JsonType::Null.into() => true ; "null type")]
    #[test_case(&json!(true), JsonType::Boolean.into() => true ; "boolean type")]
    #[test_case(&json!("test"), JsonType::String.into() => true ; "string type")]
    #[test_case(&json!([1,2]), JsonType::Array.into() => true ; "array type")]
    #[test_case(&json!({"a": 1}), JsonType::Object.into() => true ; "object type")]
    #[test_case(&json!(42), JsonType::Number.into() => true ; "number matches number")]
    #[test_case(&json!(42), JsonType::Integer.into() => true ; "int matches integer")]
    #[test_case(&json!(1.23), JsonType::Number.into() => true ; "float matches number")]
    #[test_case(&json!(1.23), JsonType::Integer.into() => false ; "float doesn't match integer")]
    fn test_contains_value_type(value: &Value, set: JsonTypeSet) -> bool {
        set.contains_value_type::<crate::JsonValueBackend>(&value)
    }

    #[test]
    fn test_remove_types() {
        let set = JsonTypeSet::all().remove(JsonType::Number);
        assert!(!set.contains(JsonType::Number));
        assert!(set.contains(JsonType::Integer));
        assert_eq!(set.len(), 6);

        let empty = JsonTypeSet::empty();
        assert_eq!(empty.remove(JsonType::Boolean), empty);
    }

    #[test]
    fn test_len() {
        let empty = JsonTypeSet::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let with_string = empty.insert(JsonType::String);
        assert!(!with_string.is_empty());
        assert_eq!(with_string.len(), 1);
        assert_eq!(JsonTypeSet::all().len(), 7);
    }

    #[test]
    fn test_debug_format() {
        assert_eq!(format!("{:?}", JsonTypeSet::default()), "()");
        assert_eq!(
            format!("{:?}", JsonTypeSet::from(JsonType::String)),
            "(string)"
        );
        assert_eq!(
            format!(
                "{:?}",
                JsonTypeSet::from(JsonType::String).insert(JsonType::Number)
            ),
            "(number, string)"
        );
    }

    #[test]
    fn test_empty_iterator() {
        let set = JsonTypeSet::empty();
        let mut iter = set.iter();
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn test_single_type_iterator() {
        let set = JsonTypeSet::from(JsonType::String);
        let mut iter = set.iter();
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.next(), Some(JsonType::String));
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn test_multiple_types_iterator() {
        let set = JsonTypeSet::from(JsonType::String)
            .insert(JsonType::Number)
            .insert(JsonType::Boolean);

        let types: Vec<JsonType> = set.iter().collect();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&JsonType::String));
        assert!(types.contains(&JsonType::Number));
        assert!(types.contains(&JsonType::Boolean));

        assert_eq!(set.iter().size_hint(), (3, Some(3)));
    }

    #[test]
    fn test_all_types_iterator() {
        let set = JsonTypeSet::all();

        let types: Vec<JsonType> = set.iter().collect();
        assert_eq!(types.len(), 7);

        let mut iter = set.iter();
        assert_eq!(iter.len(), 7);
        iter.next();
        assert_eq!(iter.len(), 6);
    }
}
