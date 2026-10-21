#[cfg(test)]
mod tests {
    use crate::alphabet::*;
    use core::convert::TryFrom as _;

    #[test]
    fn detects_duplicate_start() {
        assert_eq!(
            ParseAlphabetError::DuplicatedByte(b'A'),
            Alphabet::new("AACDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_duplicate_end() {
        assert_eq!(
            ParseAlphabetError::DuplicatedByte(b'/'),
            Alphabet::new("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789//")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_duplicate_middle() {
        assert_eq!(
            ParseAlphabetError::DuplicatedByte(b'Z'),
            Alphabet::new("ABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_length() {
        assert_eq!(
            ParseAlphabetError::InvalidLength,
            Alphabet::new(
                "xxxxxxxxxABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/",
            )
            .unwrap_err()
        );
    }

    #[test]
    fn detects_padding() {
        assert_eq!(
            ParseAlphabetError::ReservedByte(b'='),
            Alphabet::new("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+=")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_unprintable() {
        // form feed
        assert_eq!(
            ParseAlphabetError::UnprintableByte(0xc),
            Alphabet::new("\x0cBCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap_err()
        );
    }

    #[test]
    fn same_as_unchecked() {
        assert_eq!(
            STANDARD,
            Alphabet::try_from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap()
        );
    }

    #[test]
    fn str_same_as_input() {
        let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let a = Alphabet::try_from(alphabet).unwrap();
        assert_eq!(alphabet, a.as_str())
    }
}
