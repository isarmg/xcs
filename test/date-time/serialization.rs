use data_serialization::{Deserialize, Serialize, de::DeserializeOwned};
use date_time::{
    Date, Month, OffsetDateTime, PlainDateTime, SignedDuration, Time, UtcDateTime, UtcOffset,
    Weekday,
};

fn round_trip<T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq>(value: T) -> String {
    let json = json_codec::to_string(&value).unwrap();
    assert_eq!(json_codec::from_str::<T>(&json).unwrap(), value, "{json}");
    json
}

#[test]
fn real_json_codec_round_trips_all_calendar_value_types() {
    let date = Date::from_calendar_date(2024, Month::February, 29).unwrap();
    let time = Time::from_hms_nano(23, 59, 58, 123_456_789).unwrap();
    let offset = UtcOffset::from_hms(5, 30, 0).unwrap();
    let date_json = round_trip(date);
    if cfg!(feature = "serialization-human-readable") {
        assert_eq!(date_json, "\"2024-02-29\"");
    } else {
        assert_eq!(date_json, "[2024,60]");
    }
    round_trip(time);
    round_trip(offset);
    round_trip(PlainDateTime::new(date, time));
    round_trip(PlainDateTime::new(date, time).assume_offset(offset));
    round_trip(UtcDateTime::from_unix_timestamp_nanos(-1).unwrap());
    round_trip(
        OffsetDateTime::from_unix_timestamp_nanos(-1)
            .unwrap()
            .to_offset(offset),
    );
    round_trip(SignedDuration::new(-1, -123_456_789));
    round_trip(Month::February);
    round_trip(Weekday::Thursday);
    round_trip(Date::MIN);
    round_trip(Date::MAX);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct UnixTimestamp {
    #[serialization(with = "date_time::data_serialization::timestamp")]
    value: OffsetDateTime,
}

#[test]
fn timestamp_adapter_handles_pre_epoch_and_rejects_overflow() {
    let value = UnixTimestamp {
        value: OffsetDateTime::from_unix_timestamp(-1).unwrap(),
    };
    assert_eq!(round_trip(value), "{\"value\":-1}");
    assert!(json_codec::from_str::<UnixTimestamp>("{\"value\":9223372036854775807}").is_err());
    assert!(json_codec::from_str::<Date>("[2023,366]").is_err());
}

#[cfg(feature = "serialization-well-known")]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Rfc3339Record {
    #[serialization(with = "date_time::data_serialization::rfc3339")]
    value: OffsetDateTime,
    #[serialization(default, with = "date_time::data_serialization::rfc3339::option")]
    optional: Option<OffsetDateTime>,
}

#[cfg(feature = "serialization-well-known")]
#[test]
fn well_known_adapters_preserve_fixed_offsets_and_optional_values() {
    let value = OffsetDateTime::from_unix_timestamp(-1)
        .unwrap()
        .to_offset(UtcOffset::from_hms(5, 30, 0).unwrap());
    let record = Rfc3339Record {
        value,
        optional: Some(value),
    };
    assert_eq!(
        round_trip(record),
        "{\"value\":\"1970-01-01T05:29:59+05:30\",\"optional\":\"1970-01-01T05:29:59+05:30\"}"
    );
    let record = Rfc3339Record {
        value: OffsetDateTime::UNIX_EPOCH,
        optional: None,
    };
    assert_eq!(
        round_trip(record),
        "{\"value\":\"1970-01-01T00:00:00Z\",\"optional\":null}"
    );
    let missing: Rfc3339Record =
        json_codec::from_str("{\"value\":\"1970-01-01T00:00:00Z\"}").unwrap();
    assert_eq!(missing.optional, None);
    assert!(json_codec::from_str::<Rfc3339Record>("{\"value\":\"2023-02-29T00:00:00Z\"}").is_err());
}
