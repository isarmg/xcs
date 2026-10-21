#[cfg(test)]
mod test {
    #[test]
    #[cfg(feature = "wrap_help")]
    fn wrap_help_last_word() {
        use super::*;

        let help = String::from("foo bar baz");
        assert_eq!(wrap(&help, 5), "foo\nbar\nbaz");
    }

    #[test]
    #[cfg(feature = "unicode")]
    fn display_width_handles_non_ascii() {
        use super::*;

        // Popular Danish tongue-twister, the name of a fruit dessert.
        let text = "rødgrød med fløde";
        assert_eq!(display_width(text), 17);
        // Note that the string width is smaller than the string
        // length. This is due to the precomposed non-ASCII letters:
        assert_eq!(text.len(), 20);
    }

    #[test]
    #[cfg(feature = "unicode")]
    fn display_width_handles_emojis() {
        use super::*;

        let text = "😂";
        // There is a single `char`...
        assert_eq!(text.chars().count(), 1);
        // but it is double-width:
        assert_eq!(display_width(text), 2);
        // This is much less than the byte length:
        assert_eq!(text.len(), 4);
    }
}
