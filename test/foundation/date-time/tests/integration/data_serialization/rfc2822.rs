use rstest::rstest;
use data_serialization::{Deserialize, Serialize};
use serde_test2::{Compact, Configure, Token, assert_tokens};
use time::OffsetDateTime;
use time::data_serialization::rfc2822;
use time_macros::datetime;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct Test {
    #[serialization(with = "rfc2822")]
    dt: OffsetDateTime,
    #[serialization(with = "rfc2822::option")]
    option_dt: Option<OffsetDateTime>,
}

#[rstest]
#[case(
    Test {
        dt: datetime!(2000-01-01 00:00:00 UTC),
        option_dt: Some(datetime!(2000-01-01 00:00:00 UTC)),
    }.compact(),
    &[
        Token::Struct {
            name: "Test",
            len: 2,
        },
        Token::Str("dt"),
        Token::BorrowedStr("Sat, 01 Jan 2000 00:00:00 +0000"),
        Token::Str("option_dt"),
        Token::Some,
        Token::BorrowedStr("Sat, 01 Jan 2000 00:00:00 +0000"),
        Token::StructEnd,
    ]
)]
fn serialize_deserialize(#[case] value: Compact<Test>, #[case] tokens: &[Token]) {
    assert_tokens(&value, tokens);
}

#[rstest]
#[case(
    r#"{"dt": "Sat, 01 Jan 2000 00:00:00 +0000", "option_dt": null}"#,
    Test {
        dt: datetime!(2000-01-01 00:00:00 UTC),
        option_dt: None,
    }
)]
fn parse_json(#[case] json: &str, #[case] expected: Test) {
    assert_eq!(json_codec::from_str::<Test>(json).ok(), Some(expected));
}
