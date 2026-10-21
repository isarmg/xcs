#[cfg(test)]
mod tests {
    use super::*;
    use data_serialization::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    enum MyEnum {
        A,
        B,
        #[serialization(rename = "c")]
        C,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Struct {
        c: String,
        b: bool,
        a: i32,
    }

    fn create_url_params<I, K, V>(values: I) -> Vec<(Arc<str>, PercentDecodedStr)>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        values
            .into_iter()
            .map(|(k, v)| (Arc::from(k.as_ref()), PercentDecodedStr::new(v).unwrap()))
            .collect()
    }

    macro_rules! check_single_value {
        ($ty:ty, $value_str:literal, $value:expr) => {
            #[allow(clippy::bool_assert_comparison)]
            {
                let url_params = create_url_params(vec![("value", $value_str)]);
                let deserializer = PathDeserializer::new(&url_params);
                assert_eq!(<$ty>::deserialize(deserializer).unwrap(), $value);
            }
        };
    }

    #[test]
    fn test_parse_single_value() {
        check_single_value!(bool, "true", true);
        check_single_value!(bool, "false", false);
        check_single_value!(i8, "-123", -123);
        check_single_value!(i16, "-123", -123);
        check_single_value!(i32, "-123", -123);
        check_single_value!(i64, "-123", -123);
        check_single_value!(i128, "123", 123);
        check_single_value!(u8, "123", 123);
        check_single_value!(u16, "123", 123);
        check_single_value!(u32, "123", 123);
        check_single_value!(u64, "123", 123);
        check_single_value!(u128, "123", 123);
        check_single_value!(f32, "123", 123.0);
        check_single_value!(f64, "123", 123.0);
        check_single_value!(String, "abc", "abc");
        check_single_value!(String, "one%20two", "one two");
        check_single_value!(&str, "abc", "abc");
        check_single_value!(&str, "one%20two", "one two");
        check_single_value!(char, "a", 'a');

        let url_params = create_url_params(vec![("a", "B")]);
        assert_eq!(
            MyEnum::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            MyEnum::B
        );

        let url_params = create_url_params(vec![("a", "1"), ("b", "2")]);
        let error_kind = i32::deserialize(PathDeserializer::new(&url_params))
            .unwrap_err()
            .kind;
        assert!(matches!(
            error_kind,
            ErrorKind::WrongNumberOfParameters {
                expected: 1,
                got: 2
            }
        ));
    }

    #[test]
    fn test_parse_seq() {
        let url_params = create_url_params(vec![("a", "1"), ("b", "true"), ("c", "abc")]);
        assert_eq!(
            <(i32, bool, String)>::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            (1, true, "abc".to_owned())
        );

        #[derive(Debug, Deserialize, Eq, PartialEq)]
        struct TupleStruct(i32, bool, String);
        assert_eq!(
            TupleStruct::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            TupleStruct(1, true, "abc".to_owned())
        );

        let url_params = create_url_params(vec![("a", "1"), ("b", "2"), ("c", "3")]);
        assert_eq!(
            <Vec<i32>>::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            vec![1, 2, 3]
        );

        let url_params = create_url_params(vec![("a", "c"), ("a", "B")]);
        assert_eq!(
            <Vec<MyEnum>>::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            vec![MyEnum::C, MyEnum::B]
        );
    }

    #[test]
    fn test_parse_seq_tuple_string_string() {
        let url_params = create_url_params(vec![("a", "foo"), ("b", "bar")]);
        assert_eq!(
            <Vec<(String, String)>>::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            vec![
                ("a".to_owned(), "foo".to_owned()),
                ("b".to_owned(), "bar".to_owned())
            ]
        );
    }

    #[test]
    fn test_parse_seq_tuple_string_parse() {
        let url_params = create_url_params(vec![("a", "1"), ("b", "2")]);
        assert_eq!(
            <Vec<(String, u32)>>::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            vec![("a".to_owned(), 1), ("b".to_owned(), 2)]
        );
    }

    #[test]
    fn test_parse_struct() {
        let url_params = create_url_params(vec![("a", "1"), ("b", "true"), ("c", "abc")]);
        assert_eq!(
            Struct::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            Struct {
                c: "abc".to_owned(),
                b: true,
                a: 1,
            }
        );
    }

    #[test]
    fn test_parse_struct_ignoring_additional_fields() {
        let url_params = create_url_params(vec![
            ("a", "1"),
            ("b", "true"),
            ("c", "abc"),
            ("d", "false"),
        ]);
        assert_eq!(
            Struct::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            Struct {
                c: "abc".to_owned(),
                b: true,
                a: 1,
            }
        );
    }

    #[test]
    fn test_parse_map() {
        let url_params = create_url_params(vec![("a", "1"), ("b", "true"), ("c", "abc")]);
        assert_eq!(
            <HashMap<String, String>>::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            [("a", "1"), ("b", "true"), ("c", "abc")]
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect()
        );
    }

    macro_rules! test_parse_error {
        (
            $params:expr,
            $ty:ty,
            $expected_error_kind:expr $(,)?
        ) => {
            let url_params = create_url_params($params);
            let actual_error_kind = <$ty>::deserialize(PathDeserializer::new(&url_params))
                .unwrap_err()
                .kind;
            assert_eq!(actual_error_kind, $expected_error_kind);
        };
    }

    #[test]
    fn test_parse_tuple_too_many_fields() {
        test_parse_error!(
            vec![("a", "abc"), ("b", "true"), ("c", "1"), ("d", "false"),],
            (&str, bool, u32),
            ErrorKind::WrongNumberOfParameters {
                got: 4,
                expected: 3,
            }
        );
    }

    #[test]
    fn test_wrong_number_of_parameters_error() {
        test_parse_error!(
            vec![("a", "1")],
            (u32, u32),
            ErrorKind::WrongNumberOfParameters {
                got: 1,
                expected: 2,
            }
        );
    }

    #[test]
    fn test_parse_error_at_key_error() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Params {
            a: u32,
        }
        test_parse_error!(
            vec![("a", "false")],
            Params,
            ErrorKind::ParseErrorAtKey {
                key: "a".to_owned(),
                value: "false".to_owned(),
                expected_type: "u32",
            }
        );
    }

    #[test]
    fn test_parse_error_at_key_error_multiple() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Params {
            a: u32,
            b: u32,
        }
        test_parse_error!(
            vec![("a", "false")],
            Params,
            ErrorKind::ParseErrorAtKey {
                key: "a".to_owned(),
                value: "false".to_owned(),
                expected_type: "u32",
            }
        );
    }

    #[test]
    fn test_parse_error_at_index_error() {
        test_parse_error!(
            vec![("a", "false"), ("b", "true")],
            (bool, u32),
            ErrorKind::ParseErrorAtIndex {
                index: 1,
                value: "true".to_owned(),
                expected_type: "u32",
            }
        );
    }

    #[test]
    fn test_parse_error_error() {
        test_parse_error!(
            vec![("a", "false")],
            u32,
            ErrorKind::ParseError {
                value: "false".to_owned(),
                expected_type: "u32",
            }
        );
    }

    #[test]
    fn test_unsupported_type_error_nested_data_structure() {
        test_parse_error!(
            vec![("a", "false")],
            Vec<Vec<u32>>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<u32>",
            }
        );
    }

    #[test]
    fn test_parse_seq_tuple_unsupported_key_type() {
        test_parse_error!(
            vec![("a", "false")],
            Vec<(u32, String)>,
            ErrorKind::Message("Unexpected key type".to_owned())
        );
    }

    #[test]
    fn test_parse_seq_wrong_tuple_length() {
        test_parse_error!(
            vec![("a", "false")],
            Vec<(String, String, String)>,
            ErrorKind::UnsupportedType {
                name: "(alloc::string::String, alloc::string::String, alloc::string::String)",
            }
        );
    }

    #[test]
    fn test_parse_seq_seq() {
        test_parse_error!(
            vec![("a", "false")],
            Vec<Vec<String>>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<alloc::string::String>",
            }
        );
    }

    #[test]
    fn test_deserialize_key_value() {
        test_parse_error!(
            vec![("id", "123123-123-123123")],
            uuid::Uuid,
            ErrorKind::DeserializeError {
                key: "id".to_owned(),
                value: "123123-123-123123".to_owned(),
                message: "UUID parsing failed: invalid group count: expected 5, found 3".to_owned(),
            }
        );
    }
}
