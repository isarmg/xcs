use data_serialization::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serialization(rename_all = "camelCase")]
pub struct Sample {
    pub field_name: String,
}
pub mod reexport {
    pub use data_serialization::*;
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serialization(crate = "crate::reexport")]
pub struct Custom {
    pub field: String,
}
#[derive_where::derive_where(Serialize, Deserialize)]
#[serialization(crate = "data_serialization")]
pub struct Wrapped<T> {
    pub field: core::marker::PhantomData<T>,
}
#[test]
fn round_trip() {
    let v = Sample {
        field_name: "hello world".into(),
    };
    let json = json_codec::to_string(&v).unwrap();
    assert_eq!(json, r#"{"fieldName":"hello world"}"#);
    assert_eq!(json_codec::from_str::<Sample>(&json).unwrap(), v);
    let form = form_codec::to_string(&v).unwrap();
    assert_eq!(form, "fieldName=hello+world");
    assert_eq!(form_codec::from_str::<Sample>(&form).unwrap(), v);
    let c = Custom { field: "ok".into() };
    assert_eq!(
        json_codec::from_str::<Custom>(&json_codec::to_string(&c).unwrap()).unwrap(),
        c
    );
    let raw = json_codec::value::RawValue::from_string("{\"a\":1}".into()).unwrap();
    assert_eq!(json_codec::to_string(&raw).unwrap(), "{\"a\":1}");
    let n: json_codec::Value = json_codec::from_str("123456789012345678901234567890").unwrap();
    assert_eq!(
        json_codec::to_string(&n).unwrap(),
        "123456789012345678901234567890"
    );
}
#[test]
fn protocol_tokens_and_error_path() {
    use toml::span::__unstable as wire;
    assert_eq!(wire::NAME, "$__serde_spanned_private_Spanned");
    assert_eq!(wire::START_FIELD, "$__serde_spanned_private_start");
    assert_eq!(wire::END_FIELD, "$__serde_spanned_private_end");
    assert_eq!(wire::VALUE_FIELD, "$__serde_spanned_private_value");
    let mut de = json_codec::Deserializer::from_str(r#"{"fieldName":1}"#);
    let err = data_serialization::error_path::deserialize::<_, Sample>(&mut de).unwrap_err();
    assert_eq!(err.path().to_string(), "fieldName");
}

flag_set::bitflags! {
 #[derive(Serialize,Deserialize,Debug,PartialEq)]
 #[serialization(transparent)]
 pub struct Bits: u32 { const A = 1; }
}
#[test]
fn bitflag_reexport() {
    let json = json_codec::to_string(&Bits::A).unwrap();
    assert_eq!(json, "\"A\"");
    assert_eq!(json_codec::from_str::<Bits>(&json).unwrap(), Bits::A);
}

#[test]
fn date_time_human_readable_round_trip_uses_owned_capability() {
    let value = date_time::OffsetDateTime::from_unix_timestamp_nanos(-1)
        .unwrap()
        .to_offset(date_time::UtcOffset::from_hms(5, 30, 0).unwrap());
    let json = json_codec::to_string(&value).unwrap();
    let restored = json_codec::from_str::<date_time::OffsetDateTime>(&json).unwrap();
    assert_eq!(restored.unix_timestamp_nanos(), -1);
    assert_eq!(restored.offset(), value.offset());
}
