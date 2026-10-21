#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn escape_meta() {
        assert_eq!(
            escape(r"\.+*?()|[]{}^$#&-~"),
            r"\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#\&\-\~".to_string()
        );
    }

    #[test]
    fn word_byte() {
        assert!(is_word_byte(b'a'));
        assert!(!is_word_byte(b'-'));
    }

    #[test]
    #[cfg(feature = "unicode-perl")]
    fn word_char() {
        assert!(is_word_character('a'), "ASCII");
        assert!(is_word_character('à'), "Latin-1");
        assert!(is_word_character('β'), "Greek");
        assert!(is_word_character('\u{11011}'), "Brahmi (Unicode 6.0)");
        assert!(is_word_character('\u{11611}'), "Modi (Unicode 7.0)");
        assert!(is_word_character('\u{11711}'), "Ahom (Unicode 8.0)");
        assert!(is_word_character('\u{17828}'), "Tangut (Unicode 9.0)");
        assert!(is_word_character('\u{1B1B1}'), "Nushu (Unicode 10.0)");
        assert!(is_word_character('\u{16E40}'), "Medefaidrin (Unicode 11.0)");
        assert!(!is_word_character('-'));
        assert!(!is_word_character('☃'));
    }

    #[test]
    #[should_panic]
    #[cfg(not(feature = "unicode-perl"))]
    fn word_char_disabled_panic() {
        assert!(is_word_character('a'));
    }

    #[test]
    #[cfg(not(feature = "unicode-perl"))]
    fn word_char_disabled_error() {
        assert!(try_is_word_character('a').is_err());
    }
}
