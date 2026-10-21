#[cfg(test)]
mod tests {
    use crate::word::{
        new_ascii_word_bound_indices, new_unicode_words_ascii, new_word_bound_indices,
    };
    use std::string::String;
    use std::vec;
    use std::vec::Vec;

    use proptest::prelude::*;

    #[test]
    fn test_syriac_abbr_mark() {
        use crate::tables::word as wd;
        let (_, _, cat) = wd::word_category('\u{70f}');
        assert_eq!(cat, wd::WC_ALetter);
    }

    #[test]
    fn test_end_of_ayah_cat() {
        use crate::tables::word as wd;
        let (_, _, cat) = wd::word_category('\u{6dd}');
        assert_eq!(cat, wd::WC_Numeric);
    }

    #[test]
    fn test_ascii_word_bound_indices_various_cases() {
        let s = "Hello, world!";
        let words: Vec<(usize, &str)> = new_ascii_word_bound_indices(s).collect();
        let expected = vec![
            (0, "Hello"), // simple letters
            (5, ","),
            (6, " "),     // space after comma
            (7, "world"), // skip comma+space, stop at '!'
            (12, "!"),    // punctuation at the end
        ];
        assert_eq!(words, expected);
    }

    #[test]
    fn test_ascii_word_indices_various_cases() {
        let s = "Hello, world! can't e.g. var1 123,456 foo_bar example.com 127.0.0.1:9090";
        let words: Vec<&str> = new_unicode_words_ascii(s).collect();
        let expected = vec![
            ("Hello"), // simple letters
            ("world"), // skip comma+space, stop at '!'
            ("can't"), // apostrophe joins letters
            ("e.g"),
            ("var1"),
            ("123,456"), // digits+comma+digits
            ("foo_bar"),
            ("example.com"),
            ("127.0.0.1"),
            ("9090"), // port number
        ];
        assert_eq!(words, expected);
    }

    /// Strategy that yields every code-point from NUL (0) to DEL (127).
    fn ascii_char() -> impl Strategy<Value = char> {
        (0u8..=127).prop_map(|b| b as char)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]
        /// Fast path must equal general path for any ASCII input.
        #[test]
        fn proptest_ascii_matches_unicode_word_indices(
            // Vec<char> → String, length 0‒99
            s in proptest::collection::vec(ascii_char(), 0..100)
                   .prop_map(|v| v.into_iter().collect::<String>())
        ) {
            let fast: Vec<(usize, &str)> = new_ascii_word_bound_indices(&s).collect();
            let uni:  Vec<(usize, &str)> = new_word_bound_indices(&s).collect();

            prop_assert_eq!(fast, uni);
        }

        /// Fast path must equal general path for any ASCII input, forwards and backwards.
        #[test]
        fn proptest_ascii_matches_unicode_word_indices_rev(
            // Vec<char> → String, length 0‒99
            s in proptest::collection::vec(ascii_char(), 0..100)
                   .prop_map(|v| v.into_iter().collect::<String>())
        ) {
            let fast_rev: Vec<(usize, &str)> = new_ascii_word_bound_indices(&s).rev().collect();
            let uni_rev : Vec<(usize, &str)> = new_word_bound_indices(&s).rev().collect();
            prop_assert_eq!(fast_rev, uni_rev);
        }
    }
}
