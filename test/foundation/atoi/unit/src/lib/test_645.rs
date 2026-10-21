#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn max_digits() {
        assert_eq!(10, i32::max_num_digits(10));
        assert_eq!(10, u32::max_num_digits(10));
        assert_eq!(19, i64::max_num_digits(10));
        assert_eq!(20, u64::max_num_digits(10));
        assert_eq!(3, u8::max_num_digits(10));
        assert_eq!(3, i8::max_num_digits(10));
    }

    #[test]
    fn max_digits_negative() {
        assert_eq!(10, i32::max_num_digits_negative(10));
        assert_eq!(0, u32::max_num_digits_negative(10));
        assert_eq!(19, i64::max_num_digits_negative(10));
        assert_eq!(0, u64::max_num_digits_negative(10));
        assert_eq!(0, u8::max_num_digits_negative(10));
        assert_eq!(3, i8::max_num_digits_negative(10));
    }

    #[test]
    fn checked_parsing() {
        assert_eq!((Some(255), 3), u8::from_radix_10_checked(b"255"));
        assert_eq!((None, 3), u8::from_radix_10_checked(b"256"));
        assert_eq!((None, 4), u8::from_radix_10_checked(b"1000"));
        assert_eq!((Some(25), 2), u8::from_radix_10_checked(b"25"));
        assert_eq!((Some(25), 2), u8::from_radix_10_checked(b"25Blub"));
    }

    #[test]
    fn checked_parsing_radix_16() {
        assert_eq!((Some(255), 2), u8::from_radix_16_checked(b"FF"));
        assert_eq!((None, 3), u8::from_radix_16_checked(b"100"));
        assert_eq!((None, 4), u8::from_radix_16_checked(b"1000"));
        assert_eq!((Some(25), 2), u8::from_radix_16_checked(b"19"));
        assert_eq!((Some(25), 2), u8::from_radix_16_checked(b"19!Blub"));
    }
}
