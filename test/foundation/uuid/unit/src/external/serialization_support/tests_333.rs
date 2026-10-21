#[cfg(test)]
    mod tests {
        use serialization_derive::*;
        use serde_test::{Configure, Token};

        use crate::Uuid;

        const BYTES: [u8; 16] = [
            0xf9, 0x16, 0x8c, 0x5e, 0xce, 0xb2, 0x4f, 0xaa, 0xb6, 0xbf, 0x32, 0x9b, 0xf3, 0x9f,
            0xa1, 0xe4,
        ];

        #[test]
        fn test_compact_option() {
            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            struct Field {
                #[serialization(with = "crate::data_serialization::compact::option")]
                id: Option<Uuid>,
            }

            let present = Field {
                id: Some(Uuid::from_bytes(BYTES)),
            };
            let absent = Field { id: None };
            let tokens = [
                Token::Struct {
                    name: "Field",
                    len: 1,
                },
                Token::Str("id"),
                Token::Some,
                Token::Tuple { len: 16 },
                Token::U8(BYTES[0]),
                Token::U8(BYTES[1]),
                Token::U8(BYTES[2]),
                Token::U8(BYTES[3]),
                Token::U8(BYTES[4]),
                Token::U8(BYTES[5]),
                Token::U8(BYTES[6]),
                Token::U8(BYTES[7]),
                Token::U8(BYTES[8]),
                Token::U8(BYTES[9]),
                Token::U8(BYTES[10]),
                Token::U8(BYTES[11]),
                Token::U8(BYTES[12]),
                Token::U8(BYTES[13]),
                Token::U8(BYTES[14]),
                Token::U8(BYTES[15]),
                Token::TupleEnd,
                Token::StructEnd,
            ];
            serde_test::assert_tokens(&present, &tokens);
            let none_tokens = [
                Token::Struct {
                    name: "Field",
                    len: 1,
                },
                Token::Str("id"),
                Token::None,
                Token::StructEnd,
            ];
            serde_test::assert_tokens(&absent, &none_tokens);
        }

        #[test]
        fn test_serialize_compact() {
            #[derive(Serialize, Debug, Deserialize, PartialEq)]
            struct UuidContainer {
                #[serialization(with = "crate::data_serialization::compact")]
                u: crate::Uuid,
            }

            let uuid_bytes = b"F9168C5E-CEB2-4F";
            let container = UuidContainer {
                u: crate::Uuid::from_slice(uuid_bytes).unwrap(),
            };

            // more complex because of the struct wrapping the actual UUID
            // serialization
            serde_test::assert_tokens(
                &container.compact(),
                &[
                    serde_test::Token::Struct {
                        name: "UuidContainer",
                        len: 1,
                    },
                    serde_test::Token::Str("u"),
                    serde_test::Token::Tuple { len: 16 },
                    serde_test::Token::U8(uuid_bytes[0]),
                    serde_test::Token::U8(uuid_bytes[1]),
                    serde_test::Token::U8(uuid_bytes[2]),
                    serde_test::Token::U8(uuid_bytes[3]),
                    serde_test::Token::U8(uuid_bytes[4]),
                    serde_test::Token::U8(uuid_bytes[5]),
                    serde_test::Token::U8(uuid_bytes[6]),
                    serde_test::Token::U8(uuid_bytes[7]),
                    serde_test::Token::U8(uuid_bytes[8]),
                    serde_test::Token::U8(uuid_bytes[9]),
                    serde_test::Token::U8(uuid_bytes[10]),
                    serde_test::Token::U8(uuid_bytes[11]),
                    serde_test::Token::U8(uuid_bytes[12]),
                    serde_test::Token::U8(uuid_bytes[13]),
                    serde_test::Token::U8(uuid_bytes[14]),
                    serde_test::Token::U8(uuid_bytes[15]),
                    serde_test::Token::TupleEnd,
                    serde_test::Token::StructEnd,
                ],
            )
        }
    }
