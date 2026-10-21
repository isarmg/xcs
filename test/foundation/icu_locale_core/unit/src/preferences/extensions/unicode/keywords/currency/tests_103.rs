#[cfg(test)]
mod tests {
    use super::*;
    use tinystr::tinystr;

    #[test]
    fn test_valid_currency_types() {
        let valid = [
            ("USD", "usd", "USD"),
            ("uSd", "usd", "USD"),
            ("usd", "usd", "USD"),
            ("EUR", "eur", "EUR"),
            ("JPY", "jpy", "JPY"),
        ];
        for (input, expected_subtag, expected_iso) in valid {
            let parsed = CurrencyType::try_from_str(input).unwrap();
            let expected_ts_iso = TinyAsciiStr::<3>::try_from_str(expected_iso).unwrap();
            assert_eq!(parsed.as_str(), expected_subtag);
            assert_eq!(parsed.iso_code(), expected_ts_iso);
            assert_eq!(parsed, input.parse::<CurrencyType>().unwrap());
        }
    }

    #[test]
    fn test_invalid_currency_types() {
        let invalid = [
            "", "U", "US", "USDDD", "US1", "123", "U$D", " US", "US ", "ÉUR",
        ];
        for input in invalid {
            assert!(CurrencyType::try_from_str(input).is_err());
            assert!(input.parse::<CurrencyType>().is_err());
        }
    }
}
