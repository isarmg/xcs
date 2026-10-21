#[cfg(test)]
mod tests {
    use crate::automata::util::syntax;

    use super::*;

    #[track_caller]
    fn assert_strip(pattern: &str, suffix: &str, expected: Option<&str>) {
        let hir = syntax::parse(pattern).unwrap();
        let got = strip_literal_suffix(&hir, suffix.as_bytes());
        let expected = expected.map(|pattern| syntax::parse(pattern).unwrap());
        assert_eq!(expected, got);
    }

    #[test]
    fn literal() {
        assert_strip("foobar", "bar", Some("foo"));
        assert_strip("foobar", "foobar", Some(""));
        assert_strip("foobar", "", Some("foobar"));
    }

    #[test]
    fn capture() {
        assert_strip(r".+foo(bar)", "foobar", Some(r".+"));
    }

    #[test]
    fn concat() {
        assert_strip(r".+(?:ab){2}cd", "ababcd", Some(r".+"));
        assert_strip(r"a(?:)cd", "acd", Some(r""));
        assert_strip(r"az{0}cd", "acd", Some(r""));
        assert_strip(r"az{2}cd", "acd", None);
        assert_strip(r"az{0,2}cd", "acd", None);
    }

    #[test]
    fn repetition() {
        assert_strip(r"x(?:ab){3}", "bab", Some("xaba"));
        assert_strip(r"x(?:ab){2,}", "bab", Some(r"x(?:ab)*a"));
    }

    #[test]
    fn mismatch() {
        assert_strip("foobar", "baz", None);
        assert_strip("foobar", "xfoobar", None);
        assert_strip(r"foo[a-z]", "a", None);
    }

    #[test]
    fn suffix_must_be_adjacent() {
        assert_strip(r"foo(xbar)", "foobar", None);
    }

    #[test]
    fn repetition_must_be_required() {
        assert_strip(r"x(?:ab)*cd", "abcd", None);
    }

    #[track_caller]
    fn assert_fixed_length_prefix(yes: bool, pattern: &str, suffix: &[u8]) {
        let hir = syntax::parse(pattern).unwrap();
        assert_eq!(
            yes,
            strip_literal_suffix(&hir, suffix)
                .map_or(false, |hir| prefix::hir_has_fixed_length(&hir)),
        );
    }

    #[track_caller]
    fn assert_internal_suffix(yes: bool, pattern: &str, suffix: &[u8]) {
        let hir = syntax::parse(pattern).unwrap();
        assert_eq!(
            yes,
            strip_literal_suffix(&hir, suffix).map_or(false, |hir| {
                prefix::hir_can_contain_literal(&hir, suffix)
            }),
        );
    }

    #[test]
    fn reverse_suffix_accepts_prefix_without_internal_suffix() {
        assert_internal_suffix(false, r"\d+XYZ", b"XYZ");
        assert_internal_suffix(false, r"[a-q][^u-z]{13}x", b"x");
    }

    #[test]
    fn reverse_suffix_accepts_fixed_length_prefix() {
        assert_fixed_length_prefix(true, r"(?:ab|cd)XYZ", b"XYZ");
        assert_internal_suffix(true, r"[A-Z][0-9]XYZ", b"XYZ");
        assert_fixed_length_prefix(true, r"[A-Z][0-9]XYZ", b"XYZ");
    }

    #[test]
    fn reverse_suffix_fixed_length_prefix_is_conservative() {
        assert_fixed_length_prefix(false, r"a{1,3}yy", b"yy");
        assert_fixed_length_prefix(false, r"a*yy", b"yy");
    }
}
