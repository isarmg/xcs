#[cfg(test)]
mod test {
    use super::*;

    struct Str<'a>(&'a str);
    struct ReadStr<'a>(io::Cursor<&'a [u8]>);

    impl<'a> fmt::Debug for Str<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl<'a> fmt::Display for Str<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl<'a> io::Read for ReadStr<'a> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf)
        }
    }

    impl Str<'static> {
        fn hello_world() -> Self {
            Self::new("hello world")
        }
    }

    impl<'a> Str<'a> {
        fn new(s: &'a str) -> Self {
            Str(s)
        }

        fn to_reader(self) -> ReadStr<'a> {
            ReadStr(io::Cursor::new(self.0.as_bytes()))
        }
    }

    fn test_debug_matches(new_pattern: impl Fn(&str) -> Result<Pattern, BuildError>) {
        let pat = new_pattern("hello world").unwrap();
        assert!(pat.debug_matches(&Str::hello_world()));

        let pat = new_pattern("hel+o w[orl]{3}d").unwrap();
        assert!(pat.debug_matches(&Str::hello_world()));

        let pat = new_pattern("goodbye world").unwrap();
        assert_eq!(pat.debug_matches(&Str::hello_world()), false);
    }

    fn test_display_matches(new_pattern: impl Fn(&str) -> Result<Pattern, BuildError>) {
        let pat = new_pattern("hello world").unwrap();
        assert!(pat.display_matches(&Str::hello_world()));

        let pat = new_pattern("hel+o w[orl]{3}d").unwrap();
        assert!(pat.display_matches(&Str::hello_world()));

        let pat = new_pattern("goodbye world").unwrap();
        assert_eq!(pat.display_matches(&Str::hello_world()), false);
    }

    fn test_reader_matches(new_pattern: impl Fn(&str) -> Result<Pattern, BuildError>) {
        let pat = new_pattern("hello world").unwrap();
        assert!(pat
            .read_matches(Str::hello_world().to_reader())
            .expect("no io error should occur"));

        let pat = new_pattern("hel+o w[orl]{3}d").unwrap();
        assert!(pat
            .read_matches(Str::hello_world().to_reader())
            .expect("no io error should occur"));

        let pat = new_pattern("goodbye world").unwrap();
        assert_eq!(
            pat.read_matches(Str::hello_world().to_reader())
                .expect("no io error should occur"),
            false
        );
    }

    fn test_debug_rep_patterns(new_pattern: impl Fn(&str) -> Result<Pattern, BuildError>) {
        let pat = new_pattern("a+b").unwrap();
        assert!(pat.debug_matches(&Str::new("ab")));
        assert!(pat.debug_matches(&Str::new("aaaab")));
        assert!(pat.debug_matches(&Str::new("aaaaaaaaaab")));
        assert_eq!(pat.debug_matches(&Str::new("b")), false);
        assert_eq!(pat.debug_matches(&Str::new("abb")), false);
        assert_eq!(pat.debug_matches(&Str::new("aaaaabb")), false);
    }

    mod anchored {
        use super::*;
        #[test]
        fn debug_matches() {
            test_debug_matches(Pattern::new_anchored)
        }

        #[test]
        fn display_matches() {
            test_display_matches(Pattern::new_anchored)
        }

        #[test]
        fn reader_matches() {
            test_reader_matches(Pattern::new_anchored)
        }

        #[test]
        fn debug_rep_patterns() {
            test_debug_rep_patterns(Pattern::new_anchored)
        }

        // === anchored behavior =============================================
        // Tests that anchored patterns match each input type only beginning at
        // the first character.
        fn test_is_anchored(f: impl Fn(&Pattern, Str) -> bool) {
            let pat = Pattern::new_anchored("a+b").unwrap();
            assert!(f(&pat, Str::new("ab")));
            assert!(f(&pat, Str::new("aaaab")));
            assert!(f(&pat, Str::new("aaaaaaaaaab")));
            assert!(!f(&pat, Str::new("bab")));
            assert!(!f(&pat, Str::new("ffab")));
            assert!(!f(&pat, Str::new("qqqqqqqaaaaab")));
        }

        #[test]
        fn debug_is_anchored() {
            test_is_anchored(|pat, input| pat.debug_matches(&input))
        }

        #[test]
        fn display_is_anchored() {
            test_is_anchored(|pat, input| pat.display_matches(&input));
        }

        #[test]
        fn reader_is_anchored() {
            test_is_anchored(|pat, input| {
                pat.read_matches(input.to_reader())
                    .expect("no io error occurs")
            });
        }

        // === explicitly unanchored =========================================
        // Tests that if an "anchored" pattern begins with `.*?`, it matches as
        // though it was unanchored.
        fn test_explicitly_unanchored(f: impl Fn(&Pattern, Str) -> bool) {
            let pat = Pattern::new_anchored(".*?a+b").unwrap();
            assert!(f(&pat, Str::new("ab")));
            assert!(f(&pat, Str::new("aaaab")));
            assert!(f(&pat, Str::new("aaaaaaaaaab")));
            assert!(f(&pat, Str::new("bab")));
            assert!(f(&pat, Str::new("ffab")));
            assert!(f(&pat, Str::new("qqqqqqqaaaaab")));
        }

        #[test]
        fn debug_explicitly_unanchored() {
            test_explicitly_unanchored(|pat, input| pat.debug_matches(&input))
        }

        #[test]
        fn display_explicitly_unanchored() {
            test_explicitly_unanchored(|pat, input| pat.display_matches(&input));
        }

        #[test]
        fn reader_explicitly_unanchored() {
            test_explicitly_unanchored(|pat, input| {
                pat.read_matches(input.to_reader())
                    .expect("no io error occurs")
            });
        }
    }

    mod unanchored {
        use super::*;
        #[test]
        fn debug_matches() {
            test_debug_matches(Pattern::new)
        }

        #[test]
        fn display_matches() {
            test_display_matches(Pattern::new)
        }

        #[test]
        fn reader_matches() {
            test_reader_matches(Pattern::new)
        }

        #[test]
        fn debug_rep_patterns() {
            test_debug_rep_patterns(Pattern::new)
        }

        // === anchored behavior =============================================
        // Tests that unanchored patterns match anywhere in the input stream.
        fn test_is_unanchored(f: impl Fn(&Pattern, Str) -> bool) {
            let pat = Pattern::new("a+b").unwrap();
            assert!(f(&pat, Str::new("ab")));
            assert!(f(&pat, Str::new("aaaab")));
            assert!(f(&pat, Str::new("aaaaaaaaaab")));
            assert!(f(&pat, Str::new("bab")));
            assert!(f(&pat, Str::new("ffab")));
            assert!(f(&pat, Str::new("qqqfqqqqaaaaab")));
        }

        #[test]
        fn debug_is_unanchored() {
            test_is_unanchored(|pat, input| pat.debug_matches(&input))
        }

        #[test]
        fn display_is_unanchored() {
            test_is_unanchored(|pat, input| pat.display_matches(&input));
        }

        #[test]
        fn reader_is_unanchored() {
            test_is_unanchored(|pat, input| {
                pat.read_matches(input.to_reader())
                    .expect("no io error occurs")
            });
        }
    }
}
