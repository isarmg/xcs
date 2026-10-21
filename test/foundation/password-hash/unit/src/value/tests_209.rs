#[cfg(test)]
mod tests {
    use super::{Error, InvalidValue, Value};

    // Invalid value examples
    const INVALID_CHAR: &str = "x;y";
    const INVALID_TOO_LONG: &str =
        "01234567891123456789212345678931234567894123456785234567896234567";
    const INVALID_CHAR_AND_TOO_LONG: &str =
        "0!234567891123456789212345678931234567894123456785234567896234567";

    //
    // Decimal parsing tests
    //

    #[test]
    fn decimal_value() {
        let valid_decimals = &[("0", 0u32), ("1", 1u32), ("4294967295", u32::MAX)];

        for &(s, i) in valid_decimals {
            let value = Value::new(s).unwrap();
            assert!(value.is_decimal());
            assert_eq!(value.decimal().unwrap(), i)
        }
    }

    #[test]
    fn reject_decimal_with_leading_zero() {
        let value = Value::new("01").unwrap();
        let err = u32::try_from(value).err().unwrap();
        assert!(matches!(
            err,
            Error::ParamValueInvalid(InvalidValue::InvalidFormat)
        ));
    }

    #[test]
    fn reject_overlong_decimal() {
        let value = Value::new("4294967296").unwrap();
        let err = u32::try_from(value).err().unwrap();
        assert_eq!(err, Error::ParamValueInvalid(InvalidValue::InvalidFormat));
    }

    #[test]
    fn reject_negative() {
        let value = Value::new("-1").unwrap();
        let err = u32::try_from(value).err().unwrap();
        assert!(matches!(
            err,
            Error::ParamValueInvalid(InvalidValue::InvalidChar(_))
        ));
    }

    //
    // String parsing tests
    //

    #[test]
    fn string_value() {
        let valid_examples = [
            "",
            "X",
            "x",
            "xXx",
            "a+b.c-d",
            "1/2",
            "01234567891123456789212345678931",
        ];

        for &example in &valid_examples {
            let value = Value::new(example).unwrap();
            assert_eq!(value.as_str(), example);
        }
    }

    #[test]
    fn reject_invalid_char() {
        let err = Value::new(INVALID_CHAR).err().unwrap();
        assert!(matches!(
            err,
            Error::ParamValueInvalid(InvalidValue::InvalidChar(_))
        ));
    }

    #[test]
    fn reject_too_long() {
        let err = Value::new(INVALID_TOO_LONG).err().unwrap();
        assert_eq!(err, Error::ParamValueInvalid(InvalidValue::TooLong));
    }

    #[test]
    fn reject_invalid_char_and_too_long() {
        let err = Value::new(INVALID_CHAR_AND_TOO_LONG).err().unwrap();
        assert_eq!(err, Error::ParamValueInvalid(InvalidValue::TooLong));
    }
}
