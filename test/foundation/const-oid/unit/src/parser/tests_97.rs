#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::Parser;
    use crate::Error;

    #[test]
    fn parse() {
        let oid = Parser::parse("1.23.456").unwrap().finish().unwrap();
        assert_eq!(oid, "1.23.456".parse().unwrap());
    }

    #[test]
    fn reject_empty_string() {
        assert_eq!(Parser::parse("").err().unwrap(), Error::Empty);
    }

    #[test]
    fn reject_non_digits() {
        assert_eq!(
            Parser::parse("X").err().unwrap(),
            Error::DigitExpected { actual: b'X' }
        );

        assert_eq!(
            Parser::parse("1.2.X").err().unwrap(),
            Error::DigitExpected { actual: b'X' }
        );
    }

    #[test]
    fn reject_trailing_dot() {
        assert_eq!(Parser::parse("1.23.").err().unwrap(), Error::TrailingDot);
    }
}
