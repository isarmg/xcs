use date_time::{
    Date, Month, OffsetDateTime, PlainDateTime, SignedDuration, Time, UtcDateTime, UtcOffset,
    Weekday,
};

#[test]
fn calendar_leap_centuries_ordinal_iso_week_and_julian_round_trips() {
    for year in [-400, -100, -1, 0, 1, 1900, 2000, 2024, 2100, 2400] {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        assert_eq!(
            Date::from_calendar_date(year, Month::February, 29).is_ok(),
            leap
        );
        for month in [Month::January, Month::February, Month::December] {
            let date = Date::from_calendar_date(year, month, 28).unwrap();
            let (y, ordinal) = date.to_ordinal_date();
            assert_eq!(Date::from_ordinal_date(y, ordinal).unwrap(), date);
            assert_eq!(Date::from_julian_day(date.to_julian_day()).unwrap(), date);
            let (y, week, weekday) = date.to_iso_week_date();
            assert_eq!(Date::from_iso_week_date(y, week, weekday).unwrap(), date);
        }
    }
    let new_year = Date::from_calendar_date(2021, Month::January, 1).unwrap();
    assert_eq!(new_year.to_iso_week_date(), (2020, 53, Weekday::Friday));
    let leap_day = Date::from_calendar_date(2024, Month::February, 29).unwrap();
    assert_eq!(
        (leap_day + SignedDuration::days(1)).to_calendar_date(),
        (2024, Month::March, 1)
    );
}

#[test]
fn utc_fixed_offset_and_pre_epoch_nanoseconds_preserve_the_instant() {
    let offset = UtcOffset::from_hms(5, 30, 0).unwrap();
    for nanos in [
        -86_400_000_000_001i128,
        -1_000_000_001,
        -1,
        0,
        1,
        1_000_000_001,
    ] {
        let utc = OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap();
        let shifted = utc.to_offset(offset);
        assert_eq!(shifted.unix_timestamp_nanos(), nanos);
        assert_eq!(shifted.offset(), offset);
        assert_eq!(shifted.to_offset(UtcOffset::UTC), utc);
        assert_eq!(
            shifted.to_utc(),
            UtcDateTime::from_unix_timestamp_nanos(nanos).unwrap()
        );
    }
    let before_epoch = OffsetDateTime::from_unix_timestamp_nanos(-1).unwrap();
    assert_eq!(before_epoch.to_calendar_date(), (1969, Month::December, 31));
    assert_eq!(before_epoch.to_hms_nano(), (23, 59, 59, 999_999_999));
    let shifted = before_epoch.to_offset(offset);
    assert_eq!(shifted.to_calendar_date(), (1970, Month::January, 1));
    assert_eq!(shifted.to_hms_nano(), (5, 29, 59, 999_999_999));
}

#[test]
fn invalid_components_and_checked_arithmetic_fail_without_wrapping() {
    let error = Date::from_calendar_date(1900, Month::February, 29).unwrap_err();
    assert_eq!(error.name(), "day");
    assert!(error.is_conditional());
    assert!(Date::from_calendar_date(2024, Month::April, 31).is_err());
    assert!(Date::from_ordinal_date(2023, 366).is_err());
    assert!(Date::from_iso_week_date(2021, 53, Weekday::Monday).is_err());
    assert!(Time::from_hms(24, 0, 0).is_err());
    assert!(Time::from_hms(23, 59, 60).is_err());
    assert!(Time::from_hms_nano(0, 0, 0, 1_000_000_000).is_err());
    assert_eq!(
        UtcOffset::from_hms(25, 59, 59).unwrap().whole_seconds(),
        93_599
    );
    assert_eq!(
        UtcOffset::from_whole_seconds(-93_599).unwrap().as_hms(),
        (-25, -59, -59)
    );
    assert!(UtcOffset::from_hms(26, 0, 0).is_err());
    assert!(UtcOffset::from_whole_seconds(-93_600).is_err());
    assert!(Date::MAX.checked_add(SignedDuration::days(1)).is_none());
    assert!(Date::MIN.checked_sub(SignedDuration::days(1)).is_none());
    assert!(
        PlainDateTime::MAX
            .checked_add(SignedDuration::nanoseconds(1))
            .is_none()
    );
    assert!(
        PlainDateTime::MIN
            .checked_sub(SignedDuration::nanoseconds(1))
            .is_none()
    );
    assert!(
        SignedDuration::MAX
            .checked_add(SignedDuration::nanoseconds(1))
            .is_none()
    );
    assert!(OffsetDateTime::from_unix_timestamp(i64::MAX).is_err());
    assert!(OffsetDateTime::from_unix_timestamp_nanos(i128::MIN).is_err());
    assert!(
        UtcDateTime::MAX
            .checked_to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            .is_none()
    );
}

#[test]
fn configured_year_bounds_are_real() {
    let bound = if cfg!(feature = "large-dates") {
        999_999
    } else {
        9_999
    };
    assert_eq!(Date::MIN.year(), -bound);
    assert_eq!(Date::MAX.year(), bound);
    assert!(Date::from_calendar_date(-bound, Month::January, 1).is_ok());
    assert!(Date::from_calendar_date(bound, Month::December, 31).is_ok());
    assert!(Date::from_calendar_date(-bound - 1, Month::January, 1).is_err());
    assert!(Date::from_calendar_date(bound + 1, Month::January, 1).is_err());
}

#[cfg(feature = "parsing")]
#[test]
fn rfc3339_parsing_rejects_invalid_calendar_offset_and_trailing_data() {
    use date_time::format_description::well_known::Rfc3339;
    for invalid in [
        "2023-02-29T00:00:00Z",
        "2024-04-31T00:00:00Z",
        "2024-01-01T24:00:00Z",
        "2024-01-01T00:00:00+24:00",
        "2024-01-01T00:00:00Ztrailing",
    ] {
        assert!(
            OffsetDateTime::parse(invalid, &Rfc3339).is_err(),
            "{invalid}"
        );
    }
    let parsed = OffsetDateTime::parse("1970-01-01T05:29:59.999999999+05:30", &Rfc3339).unwrap();
    assert_eq!(parsed.unix_timestamp_nanos(), -1);
    assert_eq!(parsed.offset(), UtcOffset::from_hms(5, 30, 0).unwrap());
}

#[cfg(all(feature = "formatting", feature = "parsing"))]
#[test]
fn format_parse_utc_offset_pre_epoch_and_custom_calendar() {
    use date_time::format_description::{self, well_known::Rfc3339};
    for text in [
        "1970-01-01T00:00:00Z",
        "1970-01-01T05:30:00+05:30",
        "1969-12-31T23:59:59.123456789Z",
        "1969-12-31T23:59:59.000000001Z",
        "1970-01-01T05:30:00.1+05:30",
        "1970-01-01T05:30:00.001+05:30",
        "2000-02-29T12:34:56+05:30",
    ] {
        let value = OffsetDateTime::parse(text, &Rfc3339).unwrap();
        assert_eq!(value.format(&Rfc3339).unwrap(), text);
        let mut bytes = Vec::new();
        assert_eq!(value.format_into(&mut bytes, &Rfc3339).unwrap(), text.len());
        assert_eq!(bytes, text.as_bytes());
    }
    let custom =
        format_description::parse_borrowed::<3>("[year]-[month]-[day] [hour]:[minute]:[second]")
            .unwrap();
    let value = PlainDateTime::new(
        Date::from_calendar_date(2000, Month::February, 29).unwrap(),
        Time::from_hms(12, 34, 56).unwrap(),
    );
    assert_eq!(value.format(&custom).unwrap(), "2000-02-29 12:34:56");
    assert_eq!(
        PlainDateTime::parse("2000-02-29 12:34:56", &custom).unwrap(),
        value
    );
    assert!(format_description::parse_borrowed::<3>("[not_a_component]").is_err());
    assert!(
        value
            .assume_offset(UtcOffset::from_hms(5, 30, 1).unwrap())
            .format(&Rfc3339)
            .is_err()
    );
    assert!(
        Date::from_calendar_date(-1, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc()
            .format(&Rfc3339)
            .is_err()
    );
}
