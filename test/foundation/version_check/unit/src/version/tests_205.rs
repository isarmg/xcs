#[cfg(test)]
mod tests {
    use super::Version;

    macro_rules! assert_to_mmp {
        // We don't use `.into::<Option<_>>` because it's not available in 1.0.
        // We don't use the message part of `assert!` for the same reason.
        ($s:expr, None) => (
            assert_eq!(Version::parse($s), None);
        );
        ($s:expr, $mmp:expr) => (
            assert_eq!(Version::parse($s).map(|v| v.to_mmp()), Some($mmp));
        )
    }

    macro_rules! assert_from_mmp {
        (($x:expr, $y:expr, $z:expr) => $s:expr) => {
            assert_eq!(Some(Version::from_mmp($x, $y, $z)), Version::parse($s));
        };
    }

    #[test]
    fn test_str_to_mmp() {
        assert_to_mmp!("1", (1, 0, 0));
        assert_to_mmp!("1.2", (1, 2, 0));
        assert_to_mmp!("1.18.0", (1, 18, 0));
        assert_to_mmp!("3.19.0", (3, 19, 0));
        assert_to_mmp!("1.19.0-nightly", (1, 19, 0));
        assert_to_mmp!("1.12.2349", (1, 12, 2349));
        assert_to_mmp!("0.12", (0, 12, 0));
        assert_to_mmp!("1.12.5", (1, 12, 5));
        assert_to_mmp!("1.12", (1, 12, 0));
        assert_to_mmp!("1", (1, 0, 0));
        assert_to_mmp!("1.4.4-nightly (d84693b93 2017-07-09))", (1, 4, 4));
        assert_to_mmp!("1.58879.4478-dev", (1, 58879, 4478));
        assert_to_mmp!("1.58879.4478-dev (d84693b93 2017-07-09))", (1, 58879, 4478));
    }

    #[test]
    fn test_malformed() {
        assert_to_mmp!("1.65536.2", None);
        assert_to_mmp!("-1.2.3", None);
        assert_to_mmp!("1. 2", None);
        assert_to_mmp!("", None);
        assert_to_mmp!(" ", None);
        assert_to_mmp!(".", None);
        assert_to_mmp!("one", None);
        assert_to_mmp!("1.", None);
        assert_to_mmp!("1.2.3.4.5.6", None);
    }

    #[test]
    fn test_from_mmp() {
        assert_from_mmp!((1, 18, 0) => "1.18.0");
        assert_from_mmp!((3, 19, 0) => "3.19.0");
        assert_from_mmp!((1, 19, 0) => "1.19.0");
        assert_from_mmp!((1, 12, 2349) => "1.12.2349");
        assert_from_mmp!((0, 12, 0) => "0.12");
        assert_from_mmp!((1, 12, 5) => "1.12.5");
        assert_from_mmp!((1, 12, 0) => "1.12");
        assert_from_mmp!((1, 0, 0) => "1");
        assert_from_mmp!((1, 4, 4) => "1.4.4");
        assert_from_mmp!((1, 58879, 4478) => "1.58879.4478");
    }

    #[test]
    fn test_comparisons() {
        let version = Version::parse("1.18.0").unwrap();
        assert!(version.exactly("1.18.0"));
        assert!(version.at_least("1.12.0"));
        assert!(version.at_least("1.12"));
        assert!(version.at_least("1"));
        assert!(version.at_most("1.18.1"));
        assert!(!version.exactly("1.19.0"));
        assert!(!version.exactly("1.18.1"));

        let version = Version::parse("1.20.0-nightly").unwrap();
        assert!(version.exactly("1.20.0-beta"));
        assert!(version.exactly("1.20.0-nightly"));
        assert!(version.exactly("1.20.0"));
        assert!(!version.exactly("1.19"));

        let version = Version::parse("1.3").unwrap();
        assert!(version.exactly("1.3.0"));
        assert!(version.exactly("1.3.0-stable"));
        assert!(version.exactly("1.3"));
        assert!(!version.exactly("1.5.0-stable"));

        let version = Version::parse("1").unwrap();
        assert!(version.exactly("1.0.0"));
        assert!(version.exactly("1.0"));
        assert!(version.exactly("1"));

        assert!(Version::parse("one.two.three").is_none());
    }

    macro_rules! reflexive_display {
        ($s:expr) => (
            assert_eq!(Version::parse($s).unwrap().to_string(), $s);
        )
    }

    #[test]
    fn display() {
        reflexive_display!("1.0.0");
        reflexive_display!("1.2.3");
        reflexive_display!("1.12.1438");
        reflexive_display!("1.44.0");
        reflexive_display!("2.44.0");
        reflexive_display!("23459.28923.3483");
    }
}
