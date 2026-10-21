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

    #[test]
    fn symbol_matches_char_for_all_valid_symbols() {
        for symbol in (0..=u8::MAX).filter_map(Symbol::new) {
            // treat the byte as UTF-8
            let bytes = &[symbol.as_u8()];
            let s = std::str::from_utf8(bytes).unwrap();
            assert_eq!(1, s.chars().count());

            let char = s.chars().next().unwrap();
            assert_eq!(char, symbol.as_char());
        }
    }

    #[test]
    fn alphabet_debug() {
        assert_eq!(
            r##"Alphabet { symbols: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", padding: '=' }"##,
            format!("{STANDARD:?}")
        );
    }

    #[test]
    fn alphabet_symbols() {
        assert_eq!(
            STANDARD.as_str(),
            STANDARD
                .symbols()
                .iter()
                .map(|s| s.as_char())
                .collect::<String>()
        );
    }
}
