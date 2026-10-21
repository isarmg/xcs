#[cfg(test)]
mod tests {
    use super::{Error, Salt};
    use crate::errors::InvalidValue;

    #[test]
    fn new_with_valid_min_length_input() {
        let s = "abcd";
        let salt = Salt::from_b64(s).unwrap();
        assert_eq!(salt.as_ref(), s);
    }

    #[test]
    fn new_with_valid_max_length_input() {
        let s = "012345678911234567892123456789312345678941234567";
        let salt = Salt::from_b64(s).unwrap();
        assert_eq!(salt.as_ref(), s);
    }

    #[test]
    fn reject_new_too_short() {
        for &too_short in &["", "a", "ab", "abc"] {
            let err = Salt::from_b64(too_short).err().unwrap();
            assert_eq!(err, Error::SaltInvalid(InvalidValue::TooShort));
        }
    }

    #[test]
    fn reject_new_too_long() {
        let s = "01234567891123456789212345678931234567894123456785234567896234567";
        let err = Salt::from_b64(s).err().unwrap();
        assert_eq!(err, Error::SaltInvalid(InvalidValue::TooLong));
    }

    #[test]
    fn reject_new_invalid_char() {
        let s = "01234_abcd";
        let err = Salt::from_b64(s).err().unwrap();
        assert_eq!(err, Error::SaltInvalid(InvalidValue::InvalidChar('_')));
    }
}
