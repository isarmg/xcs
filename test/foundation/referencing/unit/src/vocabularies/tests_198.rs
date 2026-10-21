#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&Vocabulary::Core, 0b0000_0001, true)]
    #[test_case(&Vocabulary::Applicator, 0b0000_0010, true)]
    #[test_case(&Vocabulary::Unevaluated, 0b0000_0100, true)]
    #[test_case(&Vocabulary::Validation, 0b0000_1000, true)]
    #[test_case(&Vocabulary::Metadata, 0b0001_0000, true)]
    #[test_case(&Vocabulary::Format, 0b0010_0000, true)]
    #[test_case(&Vocabulary::FormatAnnotation, 0b0100_0000, true)]
    #[test_case(&Vocabulary::Content, 0b1000_0000, true)]
    #[test_case(&Vocabulary::Core, 0b1111_1110, false)]
    #[test_case(&Vocabulary::Applicator, 0b1111_1101, false)]
    #[test_case(&Vocabulary::Unevaluated, 0b111_11011, false)]
    #[test_case(&Vocabulary::Validation, 0b1111_0111, false)]
    #[test_case(&Vocabulary::Metadata, 0b1110_1111, false)]
    #[test_case(&Vocabulary::Format, 0b1101_1111, false)]
    #[test_case(&Vocabulary::FormatAnnotation, 0b1011_1111, false)]
    #[test_case(&Vocabulary::Content, 0b0111_1111, false)]
    #[test_case(&Vocabulary::FormatAssertion, 0b1_0000_0000, true)]
    #[test_case(&Vocabulary::FormatAssertion, 0b0_1111_1111, false)]
    fn test_vocabulary_set(vocabulary: &Vocabulary, known: u16, expected: bool) {
        let set = VocabularySet::from_known(known);
        assert_eq!(set.contains(vocabulary), expected);
    }

    #[test]
    fn test_vocabulary_set_add_and_contains() {
        let mut set = VocabularySet::new();

        set.add(Vocabulary::Core);
        set.add(Vocabulary::Applicator);
        set.add(Vocabulary::Validation);
        set.add(Vocabulary::Metadata);
        set.add(Vocabulary::Content);

        assert!(set.contains(&Vocabulary::Core));
        assert!(set.contains(&Vocabulary::Applicator));
        assert!(set.contains(&Vocabulary::Validation));
        assert!(set.contains(&Vocabulary::Metadata));
        assert!(set.contains(&Vocabulary::Content));

        assert!(!set.contains(&Vocabulary::Unevaluated));
        assert!(!set.contains(&Vocabulary::Format));
        assert!(!set.contains(&Vocabulary::FormatAnnotation));

        set.add(Vocabulary::Unevaluated);
        set.add(Vocabulary::Format);
        set.add(Vocabulary::FormatAnnotation);

        assert!(set.contains(&Vocabulary::Unevaluated));
        assert!(set.contains(&Vocabulary::Format));
        assert!(set.contains(&Vocabulary::FormatAnnotation));
    }

    #[test]
    fn test_vocabulary_set_debug() {
        let mut set = VocabularySet::from_known(0b0001_1111); // Core, Applicator, Unevaluated, Validation, Metadata
        set.add(Vocabulary::Custom(
            uri::from_str("https://example.com/custom-vocab").unwrap(),
        ));

        assert_eq!(
            format!("{set:?}"),
            "[\"core\", \"applicator\", \"unevaluated\", \"validation\", \"meta-data\", \"https://example.com/custom-vocab\"]"
        );
    }

    #[test]
    fn test_custom_vocabulary() {
        let custom_uri = uri::from_str("https://example.com/custom-vocab").expect("Invalid URI");
        let mut set = VocabularySet::new();
        set.add(Vocabulary::Custom(custom_uri.clone()));

        assert!(set.contains(&Vocabulary::Custom(custom_uri)));
        assert!(!set.contains(&Vocabulary::Custom(
            uri::from_str("https://example.com/other-vocab").expect("Invalid URI")
        )));
    }

    #[test_case(
        &serde_json::json!({"$id": "https://json-schema.org/draft/2020-12/schema"}),
        "Some([\"core\", \"applicator\", \"unevaluated\", \"validation\", \"meta-data\", \"format-annotation\", \"content\"])"
        ; "2020-12 draft"
    )]
    #[test_case(
        &serde_json::json!({"$id": "https://json-schema.org/draft/2019-09/schema"}),
        "Some([\"core\", \"applicator\", \"validation\", \"meta-data\", \"content\"])"
        ; "2019-09 draft"
    )]
    #[test_case(
        &serde_json::json!({"$id": "https://json-schema.org/draft-07/schema"}),
        "None"
        ; "draft-07"
    )]
    #[test_case(
        &serde_json::json!({
            "$id": "https://example.com/format-assertion-dialect",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/format-assertion": true,
            }
        }),
        "Some([\"core\", \"format-assertion\"])"
        ; "format-assertion dialect"
    )]
    #[test_case(
        &serde_json::json!({
            "$id": "https://example.com/format-assertion-dialect",
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/format-assertion": false,
            }
        }),
        "Some([\"core\", \"format-assertion\"])"
        ; "optional format-assertion dialect"
    )]
    #[test_case(
        &serde_json::json!({
            "$id": "https://example.com/custom-schema",
            "$vocabulary": {
                "https://example.com/custom-vocab1": true,
                "https://example.com/custom-vocab2": true,
                "https://example.com/custom-vocab3": false,
            }
        }),
        "Some([\"https://example.com/custom-vocab1\", \"https://example.com/custom-vocab2\"])"
        ; "custom schema"
    )]
    #[test_case(
        &serde_json::json!({
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/format-assertion": true,
            }
        }),
        "Some([\"core\", \"format-assertion\"])"
        ; "no $id keyword with vocabularies"
    )]
    #[test_case(
        &serde_json::json!({}),
        "None"
        ; "no $id keyword"
    )]
    fn test_find(schema: &serde_json::Value, expected: &str) {
        let set = find(schema).expect("Invalid vocabulary");
        assert_eq!(format!("{set:?}"), expected);
    }
}
