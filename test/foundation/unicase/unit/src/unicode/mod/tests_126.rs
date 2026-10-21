#[cfg(test)]
mod tests {
    use super::Unicode;

    macro_rules! eq {
        ($left:expr, $right:expr) => {{
            assert_eq!(Unicode($left), Unicode($right));
        }};
    }

    #[test]
    fn test_ascii_folding() {
        eq!("foo bar", "FoO BAR");
    }

    #[test]
    fn test_simple_case_folding() {
        eq!("στιγμας", "στιγμασ");
    }

    #[test]
    fn test_full_case_folding() {
        eq!("ﬂour", "flour");
        eq!("Maße", "MASSE");
        eq!("ᾲ στο διάολο", "ὰι στο διάολο");
    }

    #[test]
    fn test_three_char_case_folding() {
        // Characters whose full case folding is three characters.
        eq!("ﬃ", "ffi");
        eq!("ﬄ", "FFL");
        eq!("ΐ", "ι\u{308}\u{301}");
        eq!("ᾷ", "α\u{342}ι");
        assert_eq!(Unicode("ﬃ").to_folded_case(), "ffi");
    }

    #[test]
    fn test_to_folded_case() {
        assert_eq!(Unicode("Maße").to_folded_case(), "masse");
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_ascii_folding(b: &mut ::test::Bencher) {
        b.bytes = b"foo bar".len() as u64;
        b.iter(|| eq!("foo bar", "FoO BAR"));
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_simple_case_folding(b: &mut ::test::Bencher) {
        b.bytes = "στιγμας".len() as u64;
        b.iter(|| eq!("στιγμας", "στιγμασ"));
    }
}
