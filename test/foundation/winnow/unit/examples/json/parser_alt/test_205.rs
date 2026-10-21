#[cfg(test)]
mod test {
    #[allow(clippy::useless_attribute)]
    #[allow(unused_imports)] // its dead for benches
    use super::*;

    #[allow(clippy::useless_attribute)]
    #[allow(dead_code)] // its dead for benches
    type Error = winnow::error::ContextError;

    #[test]
    fn json_string() {
        assert_eq!(string::<Error>.parse_peek("\"\""), Ok(("", "".to_owned())));
        assert_eq!(
            string::<Error>.parse_peek("\"abc\""),
            Ok(("", "abc".to_owned()))
        );
        assert_eq!(
            string::<Error>
                .parse_peek("\"abc\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0001\\u2014\u{2014}def\""),
            Ok(("", "abc\"\\/\x08\x0C\n\r\t\x01——def".to_owned())),
        );
        assert_eq!(
            string::<Error>.parse_peek("\"\\uD83D\\uDE10\""),
            Ok(("", "😐".to_owned()))
        );

        assert!(string::<Error>.parse_peek("\"").is_err());
        assert!(string::<Error>.parse_peek("\"abc").is_err());
        assert!(string::<Error>.parse_peek("\"\\\"").is_err());
        assert!(string::<Error>.parse_peek("\"\\u123\"").is_err());
        assert!(string::<Error>.parse_peek("\"\\uD800\"").is_err());
        assert!(string::<Error>.parse_peek("\"\\uD800\\uD800\"").is_err());
        assert!(string::<Error>.parse_peek("\"\\uDC00\"").is_err());
    }

    #[test]
    fn json_object() {
        use JsonValue::{Num, Object, Str};

        let input = r#"{"a":42,"b":"x"}"#;

        let expected = Object(
            vec![
                ("a".to_owned(), Num(42.0)),
                ("b".to_owned(), Str("x".to_owned())),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(json::<Error>.parse_peek(input), Ok(("", expected)));
    }

    #[test]
    fn json_array() {
        use JsonValue::{Array, Num, Str};

        let input = r#"[42,"x"]"#;

        let expected = Array(vec![Num(42.0), Str("x".to_owned())]);

        assert_eq!(json::<Error>.parse_peek(input), Ok(("", expected)));
    }

    #[test]
    fn json_whitespace() {
        use JsonValue::{Array, Boolean, Null, Num, Object, Str};

        let input = r#"
  {
    "null" : null,
    "true"  :true ,
    "false":  false  ,
    "number" : 123e4 ,
    "string" : " abc 123 " ,
    "array" : [ false , 1 , "two" ] ,
    "object" : { "a" : 1.0 , "b" : "c" } ,
    "empty_array" : [  ] ,
    "empty_object" : {   }
  }
  "#;

        assert_eq!(
            json::<Error>.parse_peek(input),
            Ok((
                "",
                Object(
                    vec![
                        ("null".to_owned(), Null),
                        ("true".to_owned(), Boolean(true)),
                        ("false".to_owned(), Boolean(false)),
                        ("number".to_owned(), Num(123e4)),
                        ("string".to_owned(), Str(" abc 123 ".to_owned())),
                        (
                            "array".to_owned(),
                            Array(vec![Boolean(false), Num(1.0), Str("two".to_owned())])
                        ),
                        (
                            "object".to_owned(),
                            Object(
                                vec![
                                    ("a".to_owned(), Num(1.0)),
                                    ("b".to_owned(), Str("c".to_owned())),
                                ]
                                .into_iter()
                                .collect()
                            )
                        ),
                        ("empty_array".to_owned(), Array(vec![]),),
                        ("empty_object".to_owned(), Object(HashMap::new()),),
                    ]
                    .into_iter()
                    .collect()
                )
            ))
        );
    }
}
