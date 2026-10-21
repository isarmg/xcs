#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Config::DEFAULT)]
    #[case(Config::DEFAULT.set_formatted_components(FC::None))]
    #[case(Config::DEFAULT.set_formatted_components(FC::Date))]
    #[case(Config::DEFAULT.set_formatted_components(FC::Time))]
    #[case(Config::DEFAULT.set_formatted_components(FC::Offset))]
    #[case(Config::DEFAULT.set_formatted_components(FC::DateTime))]
    #[case(Config::DEFAULT.set_formatted_components(FC::DateTimeOffset))]
    #[case(Config::DEFAULT.set_formatted_components(FC::TimeOffset))]
    #[case(Config::DEFAULT.set_use_separators(false))]
    #[case(Config::DEFAULT.set_use_separators(true))]
    #[case(Config::DEFAULT.set_year_is_six_digits(false))]
    #[case(Config::DEFAULT.set_year_is_six_digits(true))]
    #[case(Config::DEFAULT.set_date_kind(DateKind::Calendar))]
    #[case(Config::DEFAULT.set_date_kind(DateKind::Week))]
    #[case(Config::DEFAULT.set_date_kind(DateKind::Ordinal))]
    #[case(Config::DEFAULT.set_time_precision(TimePrecision::Hour {
        decimal_digits: None
    }))]
    #[case(Config::DEFAULT.set_time_precision(TimePrecision::Minute {
        decimal_digits: None
    }))]
    #[case(Config::DEFAULT.set_time_precision(TimePrecision::Second {
        decimal_digits: None
    }))]
    #[case(Config::DEFAULT.set_time_precision(TimePrecision::Hour {
        decimal_digits: NonZero::new(1)
    }))]
    #[case(Config::DEFAULT.set_time_precision(TimePrecision::Minute {
        decimal_digits: NonZero::new(1)
    }))]
    #[case(Config::DEFAULT.set_time_precision(TimePrecision::Second {
        decimal_digits: NonZero::new(1)
    }))]
    #[case(Config::DEFAULT.set_offset_precision(OffsetPrecision::Hour))]
    #[case(Config::DEFAULT.set_offset_precision(OffsetPrecision::Minute))]
    fn encoding_roundtrip(#[case] config: Config) {
        let encoded = config.encode();
        let decoded = Config::decode(encoded);
        assert_eq!(config.formatted_components, decoded.formatted_components);
        assert_eq!(config.use_separators, decoded.use_separators);
        assert_eq!(config.year_is_six_digits, decoded.year_is_six_digits);
        assert_eq!(config.date_kind, decoded.date_kind);
        assert_eq!(config.time_precision, decoded.time_precision);
        assert_eq!(config.offset_precision, decoded.offset_precision);
    }

    #[rstest]
    #[case(0x07_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00)]
    #[case(0x00_02_00_00_00_00_00_00_00_00_00_00_00_00_00_00)]
    #[case(0x00_00_02_00_00_00_00_00_00_00_00_00_00_00_00_00)]
    #[case(0x00_00_00_03_00_00_00_00_00_00_00_00_00_00_00_00)]
    #[case(0x00_00_00_00_03_00_00_00_00_00_00_00_00_00_00_00)]
    #[case(0x00_00_00_00_00_00_02_00_00_00_00_00_00_00_00_00)]
    #[case(0x00_00_00_00_00_00_00_01_00_00_00_00_00_00_00_00)]
    #[should_panic]
    fn decode_fail(#[case] encoded: EncodedConfig) {
        Config::decode(encoded);
    }
}
