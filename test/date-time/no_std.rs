//! Compile real date/time consumers without importing the standard library.
#![no_std]

use date_time::{Date, Month, OffsetDateTime, SignedDuration, UtcOffset};

pub fn calendar() -> Date {
    Date::from_calendar_date(2000, Month::February, 29)
        .unwrap()
        .checked_add(SignedDuration::days(1))
        .unwrap()
}

pub fn fixed_offset() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(-1)
        .unwrap()
        .to_offset(UtcOffset::from_hms(5, 30, 0).unwrap())
}

#[cfg(feature = "parsing")]
pub fn parse_without_alloc() -> OffsetDateTime {
    date_time::OffsetDateTime::parse(
        "1969-12-31T23:59:59.123456789Z",
        &date_time::format_description::well_known::Rfc3339,
    )
    .unwrap()
}

#[cfg(all(feature = "alloc", feature = "parsing"))]
pub fn parse_description_with_alloc() -> Date {
    let format =
        date_time::format_description::parse_borrowed::<3>("[year]-[month]-[day]").unwrap();
    Date::parse("2024-02-29", &format).unwrap()
}

#[cfg(feature = "serialization")]
pub fn serialization_contract() {
    fn serializable<T: data_serialization::Serialize>() {}
    serializable::<Date>();
    serializable::<OffsetDateTime>();
}
