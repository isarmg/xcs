#[cfg(test)]
mod tests {
    use crate::Draft;
    use json_codec::json;
    use test_case::test_case;

    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema"}), Draft::Draft202012; "detect Draft 2020-12")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema#"}), Draft::Draft202012; "detect Draft 2020-12 with fragment")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft/2019-09/schema"}), Draft::Draft201909; "detect Draft 2019-09")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-07/schema"}), Draft::Draft7; "detect Draft 7")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft-07/schema"}), Draft::Draft7; "detect Draft 7 https")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-06/schema"}), Draft::Draft6; "detect Draft 6")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft-06/schema"}), Draft::Draft6; "detect Draft 6 https")]
    #[test_case(&json!({"$schema": "http://json-schema.org/draft-04/schema"}), Draft::Draft4; "detect Draft 4")]
    #[test_case(&json!({"$schema": "https://json-schema.org/draft-04/schema"}), Draft::Draft4; "detect Draft 4 https")]
    #[test_case(&json!({"$schema": "http://json-schema.org/schema"}), Draft::Draft202012; "detect the version-less URI")]
    #[test_case(&json!({"$schema": "http://json-schema.org/schema#"}), Draft::Draft202012; "detect the version-less URI with fragment")]
    #[test_case(&json!({"$schema": "https://json-schema.org/schema"}), Draft::Draft202012; "detect the version-less URI https")]
    #[test_case(&json!({}), Draft::Draft7; "default to Draft 7 when no $schema")]
    fn test_detect(contents: &json_codec::Value, expected: Draft) {
        let result = Draft::Draft7.detect(contents);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_unknown_specification() {
        let draft = Draft::Draft7.detect(&json!({"$schema": "invalid"}));
        assert_eq!(draft, Draft::Unknown);
    }

    // `prefixItems` is a 2020-12 keyword; 2019-09 and earlier use array-form `items`.
    #[test_case(Draft::Draft7, false; "draft7 unknown")]
    #[test_case(Draft::Draft201909, false; "draft2019-09 unknown")]
    #[test_case(Draft::Draft202012, true; "draft2020-12 known")]
    fn prefix_items_known_only_in_2020_12(draft: Draft, expected: bool) {
        assert_eq!(draft.is_known_keyword("prefixItems"), expected);
    }

    #[test_case(Draft::Draft4; "Draft 4 stays Draft 4")]
    #[test_case(Draft::Draft6; "Draft 6 stays Draft 6")]
    #[test_case(Draft::Draft7; "Draft 7 stays Draft 7")]
    #[test_case(Draft::Draft201909; "Draft 2019-09 stays Draft 2019-09")]
    #[test_case(Draft::Draft202012; "Draft 2020-12 stays Draft 2020-12")]
    fn test_detect_no_change(draft: Draft) {
        let contents = json!({});
        let result = draft.detect(&contents);
        assert_eq!(result, draft);
    }
}
