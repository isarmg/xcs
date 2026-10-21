#[cfg(test)]
mod tests {
    use super::{Error, Ident};

    // Invalid ident examples
    const INVALID_EMPTY: &str = "";
    const INVALID_CHAR: &str = "argon2;d";
    const INVALID_TOO_LONG: &str = "012345678911234567892123456789312";
    const INVALID_CHAR_AND_TOO_LONG: &str = "0!2345678911234567892123456789312";

    #[test]
    fn parse_valid() {
        let valid_examples = ["6", "x", "argon2d", "01234567891123456789212345678931"];

        for &example in &valid_examples {
            assert_eq!(example, &*Ident::new(example).unwrap());
        }
    }

    #[test]
    fn reject_empty() {
        assert_eq!(Ident::new(INVALID_EMPTY), Err(Error::ParamNameInvalid));
    }

    #[test]
    fn reject_invalid() {
        assert_eq!(Ident::new(INVALID_CHAR), Err(Error::ParamNameInvalid));
    }

    #[test]
    fn reject_too_long() {
        assert_eq!(Ident::new(INVALID_TOO_LONG), Err(Error::ParamNameInvalid));
    }

    #[test]
    fn reject_invalid_char_and_too_long() {
        assert_eq!(
            Ident::new(INVALID_CHAR_AND_TOO_LONG),
            Err(Error::ParamNameInvalid)
        );
    }
}
