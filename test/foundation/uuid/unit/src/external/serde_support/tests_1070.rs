#[cfg(test)]
    mod tests {

        use serde::de::{self, Error};
        use serde_test::{Readable, Token};

        use super::*;

        const HYPHENATED_UUID_STR: &str = "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4";
        const BRACED_UUID_STR: &str = "{f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4}";

        #[test]
        fn test_hyphenated_option() {
            #[derive(Debug, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
            struct Field {
                #[serde(with = "crate::serde::hyphenated::option")]
                id: Option<Uuid>,
            }

            let present = Field {
                id: Some(HYPHENATED_UUID_STR.parse().unwrap()),
            };
            let absent = Field { id: None };
            let present_tokens = [
                Token::Struct {
                    name: "Field",
                    len: 1,
                },
                Token::Str("id"),
                Token::Some,
                Token::Str(HYPHENATED_UUID_STR),
                Token::StructEnd,
            ];
            let absent_tokens = [
                Token::Struct {
                    name: "Field",
                    len: 1,
                },
                Token::Str("id"),
                Token::None,
                Token::StructEnd,
            ];

            serde_test::assert_tokens(&present, &present_tokens);
            serde_test::assert_tokens(&absent, &absent_tokens);
        }

        #[test]
        fn test_serialize_as_hyphenated() {
            #[derive(serde_derive::Serialize)]
            struct Struct(#[serde(with = "super")] crate::Uuid);

            let u = Struct(Uuid::parse_str(HYPHENATED_UUID_STR).unwrap());
            serde_test::assert_ser_tokens(
                &u,
                &[
                    Token::NewtypeStruct { name: "Struct" },
                    Token::Str(HYPHENATED_UUID_STR),
                ],
            );
        }

        #[test]
        fn test_de_from_hyphenated() {
            #[derive(PartialEq, Debug, serde_derive::Deserialize)]
            struct Struct(#[serde(with = "super")] crate::Uuid);
            let s = Struct(HYPHENATED_UUID_STR.parse().unwrap());
            serde_test::assert_de_tokens::<Struct>(
                &s,
                &[
                    Token::TupleStruct {
                        name: "Struct",
                        len: 1,
                    },
                    Token::BorrowedStr(HYPHENATED_UUID_STR),
                    Token::TupleStructEnd,
                ],
            );
        }

        #[test]
        fn test_de_reject_braced() {
            #[derive(PartialEq, Debug, serde_derive::Deserialize)]
            struct Struct(#[serde(with = "super")] crate::Uuid);
            serde_test::assert_de_tokens_error::<Readable<Struct>>(
                &[
                    Token::TupleStruct {
                        name: "Struct",
                        len: 1,
                    },
                    Token::BorrowedStr(BRACED_UUID_STR),
                    Token::TupleStructEnd,
                ],
                &format!(
                    "{}",
                    de::value::Error::custom(
                        "UUID parsing failed: invalid character: found `{` at 0"
                    )
                ),
            );
        }
    }
