#[cfg(test)]
    mod tests {
        use serialization_derive::*;

        use crate::Uuid;
        use serde_test::Token;

        const BYTES: [u8; 16] = [
            0xf9, 0x16, 0x8c, 0x5e, 0xce, 0xb2, 0x4f, 0xaa, 0xb6, 0xbf, 0x32, 0x9b, 0xf3, 0x9f,
            0xa1, 0xe4,
        ];

        #[test]
        fn test_bytes_option() {
            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            struct Field {
                #[serialization(with = "crate::data_serialization::bytes::option")]
                id: Option<Uuid>,
            }

            let present = Field {
                id: Some(Uuid::from_bytes(BYTES)),
            };
            let absent = Field { id: None };
            let present_tokens = [
                Token::Struct {
                    name: "Field",
                    len: 1,
                },
                Token::Str("id"),
                Token::Some,
                Token::Bytes(&BYTES),
                Token::StructEnd,
            ];
            let none_tokens = [
                Token::Struct {
                    name: "Field",
                    len: 1,
                },
                Token::Str("id"),
                Token::None,
                Token::StructEnd,
            ];
            serde_test::assert_tokens(&present, &present_tokens);
            serde_test::assert_tokens(&absent, &none_tokens);
        }

        #[test]
        fn test_serialize_bytes() {
            #[derive(Serialize, Debug, Deserialize, PartialEq)]
            struct UuidContainer {
                #[serialization(with = "crate::data_serialization::bytes")]
                u: crate::Uuid,
            }

            let uuid_bytes = b"F9168C5E-CEB2-4F";
            let container = UuidContainer {
                u: crate::Uuid::from_slice(uuid_bytes).unwrap(),
            };

            serde_test::assert_tokens(
                &container,
                &[
                    serde_test::Token::Struct {
                        name: "UuidContainer",
                        len: 1,
                    },
                    serde_test::Token::Str("u"),
                    serde_test::Token::Bytes(uuid_bytes),
                    serde_test::Token::StructEnd,
                ],
            )
        }
    }
