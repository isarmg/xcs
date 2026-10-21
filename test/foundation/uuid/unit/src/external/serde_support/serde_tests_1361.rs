#[cfg(test)]
mod serde_tests {
    use super::*;

    use serde_test::{Compact, Configure, Readable, Token};

    #[test]
    fn test_serialize_readable_string() {
        let uuid_str = "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4";
        let u = Uuid::parse_str(uuid_str).unwrap();
        serde_test::assert_tokens(&u.readable(), &[Token::Str(uuid_str)]);
    }

    #[test]
    fn test_deserialize_readable_compact() {
        let uuid_bytes = b"F9168C5E-CEB2-4F";
        let u = Uuid::from_slice(uuid_bytes).unwrap();

        serde_test::assert_de_tokens(
            &u.readable(),
            &[
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
            ],
        );
    }

    #[test]
    fn test_deserialize_readable_bytes() {
        let uuid_bytes = b"F9168C5E-CEB2-4F";
        let u = Uuid::from_slice(uuid_bytes).unwrap();

        serde_test::assert_de_tokens(&u.readable(), &[serde_test::Token::Bytes(uuid_bytes)]);
    }

    #[test]
    fn test_serialize_hyphenated() {
        let uuid_str = "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4";
        let u = Uuid::parse_str(uuid_str).unwrap();
        serde_test::assert_ser_tokens(&u.hyphenated(), &[Token::Str(uuid_str)]);
        serde_test::assert_de_tokens(&u.hyphenated(), &[Token::Str(uuid_str)]);
    }

    #[test]
    fn test_serialize_simple() {
        let uuid_str = "f9168c5eceb24faab6bf329bf39fa1e4";
        let u = Uuid::parse_str(uuid_str).unwrap();
        serde_test::assert_ser_tokens(&u.simple(), &[Token::Str(uuid_str)]);
        serde_test::assert_de_tokens(&u.simple(), &[Token::Str(uuid_str)]);
    }

    #[test]
    fn test_serialize_urn() {
        let uuid_str = "urn:uuid:f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4";
        let u = Uuid::parse_str(uuid_str).unwrap();
        serde_test::assert_ser_tokens(&u.urn(), &[Token::Str(uuid_str)]);
        serde_test::assert_de_tokens(&u.urn(), &[Token::Str(uuid_str)]);
    }

    #[test]
    fn test_serialize_braced() {
        let uuid_str = "{f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4}";
        let u = Uuid::parse_str(uuid_str).unwrap();
        serde_test::assert_ser_tokens(&u.braced(), &[Token::Str(uuid_str)]);
        serde_test::assert_de_tokens(&u.braced(), &[Token::Str(uuid_str)]);
    }

    #[test]
    fn test_serialize_non_human_readable() {
        let uuid_bytes = b"F9168C5E-CEB2-4F";
        let u = Uuid::from_slice(uuid_bytes).unwrap();
        serde_test::assert_tokens(
            &u.compact(),
            &[serde_test::Token::Bytes(&[
                70, 57, 49, 54, 56, 67, 53, 69, 45, 67, 69, 66, 50, 45, 52, 70,
            ])],
        );
    }

    #[test]
    fn test_de_failure() {
        serde_test::assert_de_tokens_error::<Readable<Uuid>>(
            &[Token::Str("hello_world")],
            "UUID parsing failed: invalid character: found `h` at 0",
        );

        serde_test::assert_de_tokens_error::<Compact<Uuid>>(
            &[Token::Bytes(b"hello_world")],
            "UUID parsing failed: invalid length: expected 16 bytes, found 11",
        );
    }

    #[test]
    fn test_serde_non_nil_uuid() {
        let uuid_str = "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4";
        let uuid = Uuid::parse_str(uuid_str).unwrap();
        let non_nil_uuid = NonNilUuid::try_from(uuid).unwrap();

        serde_test::assert_ser_tokens(&non_nil_uuid.readable(), &[Token::Str(uuid_str)]);
        serde_test::assert_de_tokens(&non_nil_uuid.readable(), &[Token::Str(uuid_str)]);
    }
}
