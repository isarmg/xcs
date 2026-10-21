#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::ToString, vec, vec::Vec};
    #[cfg(feature = "std")]
    use std::dbg;

    // `dbg!` is only available with `libstd`; just nop it out when testing
    // with alloc only.
    #[cfg(not(feature = "std"))]
    macro_rules! dbg {
        ($x:expr) => {
            $x
        };
    }

    fn expect_parse(s: &str) -> Targets {
        match dbg!(s).parse::<Targets>() {
            Err(e) => panic!("string {:?} did not parse successfully: {}", s, e),
            Ok(e) => e,
        }
    }

    fn expect_parse_ralith(s: &str) {
        let dirs = expect_parse(s).0.into_vec();
        assert_eq!(dirs.len(), 2, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("server".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::DEBUG);
        assert_eq!(dirs[0].field_names, Vec::<String>::new());

        assert_eq!(dirs[1].target, Some("common".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::INFO);
        assert_eq!(dirs[1].field_names, Vec::<String>::new());
    }

    fn expect_parse_level_directives(s: &str) {
        let dirs = expect_parse(s).0.into_vec();
        assert_eq!(dirs.len(), 6, "\nparsed: {:#?}", dirs);

        assert_eq!(dirs[0].target, Some("crate3::mod2::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::OFF);
        assert_eq!(dirs[0].field_names, Vec::<String>::new());

        assert_eq!(dirs[1].target, Some("crate1::mod2::mod3".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::INFO);
        assert_eq!(dirs[1].field_names, Vec::<String>::new());

        assert_eq!(dirs[2].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::WARN);
        assert_eq!(dirs[2].field_names, Vec::<String>::new());

        assert_eq!(dirs[3].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::ERROR);
        assert_eq!(dirs[3].field_names, Vec::<String>::new());

        assert_eq!(dirs[4].target, Some("crate3".to_string()));
        assert_eq!(dirs[4].level, LevelFilter::TRACE);
        assert_eq!(dirs[4].field_names, Vec::<String>::new());

        assert_eq!(dirs[5].target, Some("crate2".to_string()));
        assert_eq!(dirs[5].level, LevelFilter::DEBUG);
        assert_eq!(dirs[5].field_names, Vec::<String>::new());
    }

    #[test]
    fn parse_ralith() {
        expect_parse_ralith("common=info,server=debug");
    }

    #[test]
    fn parse_ralith_uc() {
        expect_parse_ralith("common=INFO,server=DEBUG");
    }

    #[test]
    fn parse_ralith_mixed() {
        expect_parse("common=iNfo,server=dEbUg");
    }

    #[test]
    fn expect_parse_valid() {
        let dirs = expect_parse("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off")
            .0
            .into_vec();
        assert_eq!(dirs.len(), 4, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::TRACE);
        assert_eq!(dirs[0].field_names, Vec::<String>::new());

        assert_eq!(dirs[1].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::ERROR);
        assert_eq!(dirs[1].field_names, Vec::<String>::new());

        assert_eq!(dirs[2].target, Some("crate3".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::OFF);
        assert_eq!(dirs[2].field_names, Vec::<String>::new());

        assert_eq!(dirs[3].target, Some("crate2".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::DEBUG);
        assert_eq!(dirs[3].field_names, Vec::<String>::new());
    }

    #[test]
    fn parse_level_directives() {
        expect_parse_level_directives(
            "crate1::mod1=error,crate1::mod2=warn,crate1::mod2::mod3=info,\
             crate2=debug,crate3=trace,crate3::mod2::mod1=off",
        )
    }

    #[test]
    fn parse_uppercase_level_directives() {
        expect_parse_level_directives(
            "crate1::mod1=ERROR,crate1::mod2=WARN,crate1::mod2::mod3=INFO,\
             crate2=DEBUG,crate3=TRACE,crate3::mod2::mod1=OFF",
        )
    }

    #[test]
    fn parse_numeric_level_directives() {
        expect_parse_level_directives(
            "crate1::mod1=1,crate1::mod2=2,crate1::mod2::mod3=3,crate2=4,\
             crate3=5,crate3::mod2::mod1=0",
        )
    }

    #[test]
    fn targets_iter() {
        let filter = expect_parse("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off")
            .with_default(LevelFilter::WARN);

        let mut targets: Vec<_> = filter.iter().collect();
        targets.sort();

        assert_eq!(
            targets,
            vec![
                ("crate1::mod1", LevelFilter::ERROR),
                ("crate1::mod2", LevelFilter::TRACE),
                ("crate2", LevelFilter::DEBUG),
                ("crate3", LevelFilter::OFF),
            ]
        );
    }

    #[test]
    fn targets_into_iter() {
        let filter = expect_parse("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off")
            .with_default(LevelFilter::WARN);

        let mut targets: Vec<_> = filter.into_iter().collect();
        targets.sort();

        assert_eq!(
            targets,
            vec![
                ("crate1::mod1".to_string(), LevelFilter::ERROR),
                ("crate1::mod2".to_string(), LevelFilter::TRACE),
                ("crate2".to_string(), LevelFilter::DEBUG),
                ("crate3".to_string(), LevelFilter::OFF),
            ]
        );
    }

    #[test]
    fn targets_default_level() {
        let filter = expect_parse("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off");
        assert_eq!(filter.default_level(), None);

        let filter = expect_parse("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off")
            .with_default(LevelFilter::OFF);
        assert_eq!(filter.default_level(), Some(LevelFilter::OFF));

        let filter = expect_parse("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off")
            .with_default(LevelFilter::OFF)
            .with_default(LevelFilter::INFO);
        assert_eq!(filter.default_level(), Some(LevelFilter::INFO));
    }

    #[test]
    // `println!` is only available with `libstd`.
    #[cfg(feature = "std")]
    fn size_of_filters() {
        use std::println;

        fn print_sz(s: &str) {
            let filter = s.parse::<Targets>().expect("filter should parse");
            println!(
                "size_of_val({:?})\n -> {}B",
                s,
                std::mem::size_of_val(&filter)
            );
        }

        print_sz("info");

        print_sz("foo=debug");

        print_sz(
            "crate1::mod1=error,crate1::mod2=warn,crate1::mod2::mod3=info,\
            crate2=debug,crate3=trace,crate3::mod2::mod1=off",
        );
    }

    /// Test that the `fmt::Display` implementation for `Targets` emits a string
    /// that can itself be parsed as a `Targets`, and that the parsed `Targets`
    /// is equivalent to the original one.
    #[test]
    fn display_roundtrips() {
        fn test_roundtrip(s: &str) {
            let filter = expect_parse(s);
            // we don't assert that the display output is equivalent to the
            // original parsed filter string, because the `Display` impl always
            // uses lowercase level names and doesn't use the
            // target-without-level shorthand syntax. while they may not be
            // textually equivalent, though, they should still *parse* to the
            // same filter.
            let formatted = filter.to_string();
            let filter2 = match dbg!(&formatted).parse::<Targets>() {
                Ok(filter) => filter,
                Err(e) => panic!(
                    "failed to parse formatted filter string {:?}: {}",
                    formatted, e
                ),
            };
            assert_eq!(filter, filter2);
        }

        test_roundtrip("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off");
        test_roundtrip(
            "crate1::mod1=ERROR,crate1::mod2=WARN,crate1::mod2::mod3=INFO,\
        crate2=DEBUG,crate3=TRACE,crate3::mod2::mod1=OFF",
        );
        test_roundtrip(
            "crate1::mod1=error,crate1::mod2=warn,crate1::mod2::mod3=info,\
             crate2=debug,crate3=trace,crate3::mod2::mod1=off",
        );
        test_roundtrip("crate1::mod1,crate1::mod2,info");
        test_roundtrip("crate1");
        test_roundtrip("info");
    }
}
