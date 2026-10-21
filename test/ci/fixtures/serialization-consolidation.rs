use data_serialization::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Numbers {
    signed: i128,
    unsigned: u128,
    result: Result<u64, String>,
}
#[test]
fn exact_integers_and_result() {
    let value = Numbers {
        signed: i128::MIN,
        unsigned: u128::MAX,
        result: Err("bad".into()),
    };
    let json = json_codec::to_string(&value).unwrap();
    assert!(json.contains("-170141183460469231731687303715884105728"));
    assert!(json.contains("340282366920938463463374607431768211455"));
    assert!(json.contains(r#""result":{"Err":"bad"}"#));
    assert_eq!(json_codec::from_str::<Numbers>(&json).unwrap(), value);
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serialization(tag = "kind")]
enum Tagged {
    Record { id: u32 },
    Empty,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serialization(untagged)]
enum Untagged {
    Number(u64),
    Text(String),
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Flattened {
    label: String,
    #[serialization(flatten)]
    fields: std::collections::BTreeMap<String, Untagged>,
}
#[test]
fn private_content_tagged_untagged_and_flattened() {
    let tagged = Tagged::Record { id: 7 };
    assert_eq!(
        json_codec::to_string(&tagged).unwrap(),
        r#"{"kind":"Record","id":7}"#
    );
    assert_eq!(
        json_codec::from_str::<Tagged>(r#"{"kind":"Record","id":7}"#).unwrap(),
        tagged
    );
    let source = r#"{"label":"ok","count":42,"name":"item"}"#;
    let parsed: Flattened = json_codec::from_str(source).unwrap();
    assert_eq!(parsed.fields["count"], Untagged::Number(42));
    assert_eq!(parsed.fields["name"], Untagged::Text("item".into()));
    assert_eq!(
        json_codec::from_str::<Flattened>(&json_codec::to_string(&parsed).unwrap()).unwrap(),
        parsed
    );
}
#[derive(Debug, Deserialize)]
struct Borrowed<'a> {
    text: &'a str,
}
#[test]
fn borrowing_does_not_allocate_or_change_lifetime() {
    let source = r#"{"text":"borrowed"}"#;
    let parsed: Borrowed<'_> = json_codec::from_str(source).unwrap();
    assert_eq!(parsed.text, "borrowed");
    assert_eq!(parsed.text.as_ptr(), source[9..].as_ptr());
}
#[derive(Debug, Deserialize)]
struct Nested {
    #[allow(dead_code)]
    entries: Vec<std::collections::BTreeMap<String, u8>>,
}
#[test]
fn structured_error_path_preserves_original_error() {
    let mut decoder = json_codec::Deserializer::from_str(r#"{"entries":[{"limit":256}]}"#);
    let error = data_serialization::error_path::deserialize::<_, Nested>(&mut decoder).unwrap_err();
    assert_eq!(error.path().to_string(), "entries[0].limit");
    assert!(error.inner().is_data());
    assert_eq!(error.inner().line(), 1);
    assert!(error.to_string().starts_with("entries[0].limit: "));
    assert!(error.into_inner().to_string().contains("u8"));
}
mod custom {
    pub use data_serialization::*;
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serialization(crate = "crate::custom")]
struct Custom {
    enabled: bool,
}
#[test]
fn derive_can_use_explicit_reexport() {
    let value = Custom { enabled: true };
    assert_eq!(
        json_codec::from_str::<Custom>(&json_codec::to_string(&value).unwrap()).unwrap(),
        value
    );
}
#[test]
fn original_formatting_tests_cover_all_bytes() {
    for value in 0..=255u8 {
        assert_eq!(json_codec::to_string(&value).unwrap(), value.to_string());
    }
}

#[test]
fn direct_macro_consumers_use_the_unified_entry() {
    let url = url::Url::parse("https://example.com/path?q=value").unwrap();
    assert_eq!(
        json_codec::to_string(&url).unwrap(),
        "\"https://example.com/path?q=value\""
    );
    assert_eq!(
        json_codec::from_str::<url::Url>(&json_codec::to_string(&url).unwrap()).unwrap(),
        url
    );
    let value = fraction::GenericFraction::<u64>::new(1u64, 3u64);
    assert_eq!(
        json_codec::from_str::<fraction::GenericFraction<u64>>(
            &json_codec::to_string(&value).unwrap()
        )
        .unwrap(),
        value
    );
}

#[test]
fn log_levels_preserve_serialization_feature_and_wire_names() {
    assert_eq!(json_codec::to_string(&log::Level::Info).unwrap(), "\"INFO\"");
    assert_eq!(json_codec::from_str::<log::Level>("\"wArN\"").unwrap(), log::Level::Warn);
    assert_eq!(json_codec::to_string(&log::LevelFilter::Off).unwrap(), "\"OFF\"");
    assert_eq!(json_codec::from_str::<log::LevelFilter>("\"dEbUg\"").unwrap(), log::LevelFilter::Debug);
}
