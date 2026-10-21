#[cfg(test)]
mod tests {
    use super::is_quoted_string_or_char;

    #[test]
    fn test_is_quoted_string_or_char() {
        assert!(is_quoted_string_or_char("\"hello\"", false));
        assert!(!is_quoted_string_or_char("abc", false));
        assert!(!is_quoted_string_or_char("\"", false));
        assert!(!is_quoted_string_or_char("\"abc", false));
        assert!(is_quoted_string_or_char("'a'", true));
        assert!(!is_quoted_string_or_char("'abc", true));
        assert!(!is_quoted_string_or_char("abc", true));
    }
}
