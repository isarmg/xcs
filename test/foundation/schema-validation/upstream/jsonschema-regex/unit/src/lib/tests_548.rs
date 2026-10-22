#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(r"\d", "[0-9]"; "digit class")]
    #[test_case(r"\D", "[^0-9]"; "non-digit class")]
    #[test_case(r"\w", "[A-Za-z0-9_]"; "word class")]
    #[test_case(r"\W", "[^A-Za-z0-9_]"; "non-word class")]
    #[test_case(r"[\d]", "[[0-9]]"; "digit class in character set")]
    #[test_case(r"[\D]", "[[^0-9]]"; "non-digit class in character set")]
    #[test_case(r"[\w]", "[[A-Za-z0-9_]]"; "word class in character set")]
    #[test_case(r"[\W]", "[[^A-Za-z0-9_]]"; "non-word class in character set")]
    #[test_case(r"\d+\w*", "[0-9]+[A-Za-z0-9_]*"; "combination of digit and word classes")]
    #[test_case(r"\D*\W+", "[^0-9]*[^A-Za-z0-9_]+"; "combination of non-digit and non-word classes")]
    #[test_case(r"[\d\w]", "[[0-9][A-Za-z0-9_]]"; "digit and word classes in character set")]
    #[test_case(r"[^\d\w]", "[^[0-9][A-Za-z0-9_]]"; "negated digit and word classes in character set")]
    #[test_case(r"[\d\w\d\w]", "[[0-9][A-Za-z0-9_][0-9][A-Za-z0-9_]]"; "multiple replacements")]
    #[test_case(r"\cA\cB\cC", "\x01\x02\x03"; "multiple control characters")]
    #[test_case(r"foo\cIbar\cXbaz", "foo\x09bar\x18baz"; "control characters mixed with text")]
    #[test_case(r"\ca\cb\cc", "\x01\x02\x03"; "lowercase control characters")]
    #[test_case(r"^[a-z&&^b]+$", r"^[a-z\&\&^b]+$"; "class intersection syntax is literal")]
    #[test_case(r"^[a-z--b]$", r"^[a-z\--b]$"; "class difference syntax preserves ranges")]
    #[test_case(r"^[\w--z]$", r"^[[A-Za-z0-9_]\-\-z]$"; "word class escape before doubled hyphen")]
    #[test_case(r"^[\d--z]$", r"^[[0-9]\-\-z]$"; "digit class escape before doubled hyphen")]
    #[test_case(r"^[a~~b]$", r"^[a\~\~b]$"; "class symmetric difference syntax is literal")]
    #[test_case(r"^[[a]]$", r"^[\[a]]$"; "nested class opener is literal")]
    #[test_case(r"[a-z]", r"[a-z]"; "ordinary class is unchanged")]
    #[test_case(r"[\[\]]", r"[\[\]]"; "escaped brackets are unchanged")]
    #[test_case(r"[^&]", r"[^&]"; "single ampersand is unchanged")]
    #[test_case(r"a&&b", r"a&&b"; "operator syntax outside a class is unchanged")]
    fn test_ecma262_to_rust_regex(input: &str, expected: &str) {
        let result = to_rust_regex(input).unwrap();
        assert_eq!(result, expected);
    }

    const ECMA_WHITESPACE: [char; 25] = [
        '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2003}',
        '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
        '\u{2001}', '\u{2002}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}',
    ];
    const NOT_ECMA_WHITESPACE: [char; 7] = [
        '\u{85}', '\u{1c}', '\u{200b}', '\u{180e}', 'a', '0', '\u{2010}',
    ];

    #[test_case(r"\s", true; "space class")]
    #[test_case(r"\S", false; "non-space class")]
    #[test_case(r"[\s]", true; "space class in set")]
    #[test_case(r"[^\s]", false; "negated set of space class")]
    #[test_case(r"[a\s]", true; "space class with literal in set")]
    #[test_case(r"[\S]", false; "non-space class in set")]
    fn space_class_matches_ecma_whitespace(pattern: &str, matches_whitespace: bool) {
        let regex =
            regex::Regex::new(&format!("^(?:{})$", to_rust_regex(pattern).unwrap())).unwrap();
        for c in ECMA_WHITESPACE {
            assert_eq!(
                regex.is_match(&c.to_string()),
                matches_whitespace,
                "{pattern} on U+{:04X}",
                c as u32
            );
        }
        for c in NOT_ECMA_WHITESPACE {
            let expected = if matches_whitespace {
                c == 'a' && pattern.starts_with("[a")
            } else {
                true
            };
            assert_eq!(
                regex.is_match(&c.to_string()),
                expected,
                "{pattern} on U+{:04X}",
                c as u32
            );
        }
    }

    #[test]
    fn whitespace_test_set_agrees_with_predicate() {
        assert!(ECMA_WHITESPACE.into_iter().all(is_ecma_whitespace));
        assert!(!NOT_ECMA_WHITESPACE.into_iter().any(is_ecma_whitespace));
    }

    #[test_case(r"\c"; "incomplete control character")]
    #[test_case(r"\c?"; "invalid control character")]
    #[test_case(r"\mA"; "another invalid control character")]
    #[test_case(r"[a-z"; "unclosed character class")]
    #[test_case(r"(abc"; "unclosed parenthesis")]
    #[test_case(r"abc)"; "unmatched closing parenthesis")]
    #[test_case(r"a{3,2}"; "invalid quantifier range")]
    #[test_case(r"\"; "trailing backslash")]
    #[test_case(r"[a-\w]"; "invalid character range")]
    #[test_case(r"[z--]"; "invalid range ending in a hyphen")]
    fn test_invalid_regex(input: &str) {
        let result = to_rust_regex(input);
        assert!(result.is_err(), "Expected error for input: {input}");
    }

    #[test]
    fn doubled_hyphen_preserves_ecmascript_range_semantics() {
        let translated = to_rust_regex(r"^[a-z--b]$").expect("valid ECMAScript regex");
        let regex = regex::Regex::new(&translated).expect("translation compiles");

        assert!(regex.is_match("0"));
        assert!(regex.is_match("A"));
        assert!(regex.is_match("z"));
        assert!(!regex.is_match("{"));
    }

    #[test_case("^foo", Some("foo"))]
    #[test_case("^x-", Some("x-"))]
    #[test_case("^eo_band", Some("eo_band"))]
    #[test_case("^path/to", Some("path/to"))]
    #[test_case("^ABC123", Some("ABC123"))]
    #[test_case("^\\/", Some("/"); "escaped slash prefix")]
    #[test_case("^\\/path", Some("/path"); "escaped slash with suffix")]
    #[test_case("^\\$ref", Some("$ref"); "escaped dollar ref")]
    #[test_case("^\\$defs", Some("$defs"); "escaped dollar defs")]
    #[test_case("foo", None; "no anchor")]
    #[test_case("^foo$", None; "end anchor")]
    #[test_case("^\\$ref$", None; "exact match dollar ref is not a prefix")]
    #[test_case("^foo.*", None; "contains dot")]
    #[test_case("^foo+", None; "contains plus")]
    #[test_case("^foo?", None; "contains question")]
    #[test_case("^[a-z]", None; "contains bracket")]
    #[test_case("^foo|bar", None; "contains pipe")]
    #[test_case("^foo(bar)", None; "contains parens")]
    #[test_case("^foo\\d", None; "contains backslash-d")]
    fn test_pattern_as_prefix(pattern: &str, expected: Option<&str>) {
        assert_eq!(pattern_as_prefix(pattern).as_deref(), expected);
    }

    #[test_case("^foo", PatternAnalysis::Prefix("foo".into()) ; "prefix")]
    #[test_case("^x-", PatternAnalysis::Prefix("x-".into()) ; "x_prefix")]
    #[test_case("^\\$ref", PatternAnalysis::Prefix("$ref".into()) ; "escaped dollar ref prefix")]
    #[test_case("^foo$", PatternAnalysis::Exact("foo".into()) ; "exact fast path")]
    #[test_case("^\\$ref$", PatternAnalysis::Exact("$ref".into()) ; "exact escaped dollar ref")]
    #[test_case(
        "^(get|put|post|delete|options|head|patch|trace)$",
        PatternAnalysis::Alternation(vec![
            "delete".into(), "get".into(), "head".into(), "options".into(),
            "patch".into(), "post".into(), "put".into(), "trace".into(),
        ]) ; "http methods alternation"
    )]
    #[test_case(
        "^(a|b|c)$",
        PatternAnalysis::Alternation(vec!["a".into(), "b".into(), "c".into()]) ; "simple alternation sorted"
    )]
    #[test_case(
        "^(a\\/b|c)$",
        PatternAnalysis::Alternation(vec!["a/b".into(), "c".into()]) ; "alternation with escaped slash"
    )]
    #[test_case(
        "^(x\\$y|z)$",
        PatternAnalysis::Alternation(vec!["x$y".into(), "z".into()]) ; "alternation with escaped dollar"
    )]
    #[test_case("^a\\.b", PatternAnalysis::Prefix("a.b".into()) ; "escaped dot prefix")]
    #[test_case("^a\\.b$", PatternAnalysis::Exact("a.b".into()) ; "escaped dot exact")]
    #[test_case("^a\\-b\\_c", PatternAnalysis::Prefix("a-b_c".into()) ; "escaped dash underscore prefix")]
    #[test_case(
        "^(a\\.b|c\\-d)$",
        PatternAnalysis::Alternation(vec!["a.b".into(), "c-d".into()]) ; "alternation with escaped dot and dash"
    )]
    #[test_case(r"^\S*$", PatternAnalysis::NoWhitespace ; "no whitespace")]
    fn test_analyze_pattern(pattern: &str, expected: PatternAnalysis<'_>) {
        assert_eq!(analyze_pattern(pattern), Some(expected));
    }

    #[test_case(' ' ; "space")]
    #[test_case('\t' ; "tab")]
    #[test_case('\u{00a0}' ; "nbsp")]
    #[test_case('\u{2003}' ; "em space")]
    #[test_case('\u{3000}' ; "ideographic space")]
    #[test_case('\u{feff}' ; "bom")]
    fn test_is_ecma_whitespace(c: char) {
        assert!(is_ecma_whitespace(c));
    }

    #[test_case('a' ; "letter")]
    #[test_case('0' ; "digit")]
    #[test_case('\u{200b}' ; "zero width space")]
    fn test_not_ecma_whitespace(c: char) {
        assert!(!is_ecma_whitespace(c));
    }

    #[test_case("foo" ; "no anchor")]
    #[test_case("^foo.*" ; "contains dot")]
    #[test_case("^foo+" ; "contains plus")]
    #[test_case("^[a-z]" ; "contains bracket")]
    #[test_case("^(a|b^)$" ; "invalid char in alternation")]
    #[test_case("^(a\\db)$" ; "invalid escape in alternation")]
    #[test_case("^a\\-$b" ; "escaped then dollar not at end")]
    fn test_analyze_pattern_none(pattern: &str) {
        assert_eq!(analyze_pattern(pattern), None);
    }

    #[test_case("=", "="; "bare literal")]
    #[test_case("b$", "b"; "tail anchored literal")]
    #[test_case("^abc", "abc"; "prefix")]
    #[test_case("^abc$", "abc"; "exact")]
    #[test_case("^(red|green)$", "red"; "alternation")]
    #[test_case(r"^\S*$", ""; "star repetition")]
    #[test_case("", ""; "empty pattern")]
    #[test_case("a.b", "aab"; "wildcard")]
    #[test_case("a+", "a"; "plus repetition")]
    #[test_case(r"\d{3}", "000"; "counted character class")]
    #[test_case("^[a-z]+-[0-9]+$", "a-0"; "classes around a literal")]
    fn test_pattern_witness(pattern: &str, expected: &str) {
        let witness = pattern_witness(pattern).expect("has a witness");
        assert_eq!(witness, expected);
        let regex = regex::Regex::new(&to_rust_regex(pattern).expect("translates")).expect("valid");
        assert!(
            regex.is_match(&witness),
            "`{witness}` does not match `{pattern}`"
        );
    }

    #[test_case(r"(?=a)b"; "look ahead the translation rejects")]
    #[test_case("a{100}"; "more repetitions than are written out")]
    #[test_case("((a{64}){64}){64}"; "nested repetitions multiplying past the length cap")]
    fn test_pattern_without_a_witness(pattern: &str) {
        assert_eq!(pattern_witness(pattern), None);
    }
    #[test_case("" ; "empty")]
    #[test_case("plain" ; "ascii without whitespace")]
    #[test_case("\u{00e9}t\u{00e9}" ; "non-ascii without whitespace")]
    #[test_case("\u{4e2d}\u{6587}" ; "multi byte without whitespace")]
    fn no_whitespace(input: &str) {
        assert!(!contains_ecma_whitespace(input));
    }

    #[test_case(" " ; "space")]
    #[test_case("a\tb" ; "tab")]
    #[test_case("a\nb" ; "newline")]
    #[test_case("a\u{000b}b" ; "vertical tab")]
    #[test_case("a\u{000c}b" ; "form feed")]
    #[test_case("a\rb" ; "carriage return")]
    #[test_case("a\u{00a0}b" ; "no break space")]
    #[test_case("a\u{1680}b" ; "ogham space mark")]
    #[test_case("a\u{2000}b" ; "en quad")]
    #[test_case("a\u{200a}b" ; "hair space")]
    #[test_case("a\u{2028}b" ; "line separator")]
    #[test_case("a\u{2029}b" ; "paragraph separator")]
    #[test_case("a\u{202f}b" ; "narrow no break space")]
    #[test_case("a\u{205f}b" ; "medium mathematical space")]
    #[test_case("a\u{3000}b" ; "ideographic space")]
    #[test_case("a\u{feff}b" ; "zero width no break space")]
    fn has_whitespace(input: &str) {
        assert!(contains_ecma_whitespace(input));
    }

    // A byte scan must not mistake a continuation byte for whitespace.
    #[test_case("\u{0a20}" ; "leading byte matches newline value")]
    #[test_case("\u{2820}" ; "continuation byte matches space value")]
    fn multi_byte_is_not_whitespace(input: &str) {
        assert!(!contains_ecma_whitespace(input));
        assert_eq!(
            contains_ecma_whitespace(input),
            input.chars().any(is_ecma_whitespace)
        );
    }
    // The byte scan must agree with a decode for every character there is.
    #[test]
    fn agrees_with_decoding_on_every_scalar_value() {
        let mut buffer = [0_u8; 4];
        for value in 0..=0x0010_FFFF_u32 {
            let Some(c) = char::from_u32(value) else {
                continue;
            };
            let text = c.encode_utf8(&mut buffer);
            assert_eq!(
                contains_ecma_whitespace(text),
                text.chars().any(is_ecma_whitespace),
                "{c:?} (U+{value:04X})"
            );
        }
    }

    // Bytes of a multi-byte encoding are all >= 0x80, so no combination can spell an ASCII
    // whitespace byte; these sequences check that directly.
    #[test]
    fn agrees_with_decoding_on_mixed_sequences() {
        let alphabet = [
            'a',
            ' ',
            '\t',
            '\n',
            '\r',
            '\u{000b}',
            '\u{000c}',
            '\u{00a0}',
            '\u{0a20}',
            '\u{2820}',
            '\u{2000}',
            '\u{200a}',
            '\u{2028}',
            '\u{3000}',
            '\u{feff}',
            '\u{4e2d}',
            '\u{10348}',
        ];
        let mut text = String::new();
        for first in alphabet {
            for second in alphabet {
                for third in alphabet {
                    text.clear();
                    text.push(first);
                    text.push(second);
                    text.push(third);
                    assert_eq!(
                        contains_ecma_whitespace(&text),
                        text.chars().any(is_ecma_whitespace),
                        "{text:?}"
                    );
                }
            }
        }
    }
}
