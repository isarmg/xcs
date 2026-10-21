#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_match_properties() {
        let haystack = b"Hello, world!";
        let m = Match::new(haystack, 7, 12);

        assert_eq!(m.start(), 7);
        assert_eq!(m.end(), 12);
        assert_eq!(m.is_empty(), false);
        assert_eq!(m.len(), 5);
        assert_eq!(m.as_bytes(), b"world");
    }

    #[test]
    fn test_empty_match() {
        let haystack = b"";
        let m = Match::new(haystack, 0, 0);

        assert_eq!(m.is_empty(), true);
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_debug_output_valid_utf8() {
        let haystack = b"Hello, world!";
        let m = Match::new(haystack, 7, 12);
        let debug_str = format!("{m:?}");

        assert_eq!(
            debug_str,
            r#"Match { start: 7, end: 12, bytes: "world" }"#
        );
    }

    #[test]
    fn test_debug_output_invalid_utf8() {
        let haystack = b"Hello, \xFFworld!";
        let m = Match::new(haystack, 7, 13);
        let debug_str = format!("{m:?}");

        assert_eq!(
            debug_str,
            r#"Match { start: 7, end: 13, bytes: "\xffworld" }"#
        );
    }

    #[test]
    fn test_debug_output_various_unicode() {
        let haystack =
            "Hello, 😊 world! 안녕하세요? مرحبا بالعالم!".as_bytes();
        let m = Match::new(haystack, 0, haystack.len());
        let debug_str = format!("{m:?}");

        assert_eq!(
            debug_str,
            r#"Match { start: 0, end: 62, bytes: "Hello, 😊 world! 안녕하세요? مرحبا بالعالم!" }"#
        );
    }

    #[test]
    fn test_debug_output_ascii_escape() {
        let haystack = b"Hello,\tworld!\nThis is a \x1b[31mtest\x1b[0m.";
        let m = Match::new(haystack, 0, haystack.len());
        let debug_str = format!("{m:?}");

        assert_eq!(
            debug_str,
            r#"Match { start: 0, end: 38, bytes: "Hello,\tworld!\nThis is a \u{1b}[31mtest\u{1b}[0m." }"#
        );
    }

    #[test]
    fn test_debug_output_match_in_middle() {
        let haystack = b"The quick brown fox jumps over the lazy dog.";
        let m = Match::new(haystack, 16, 19);
        let debug_str = format!("{m:?}");

        assert_eq!(debug_str, r#"Match { start: 16, end: 19, bytes: "fox" }"#);
    }
}
