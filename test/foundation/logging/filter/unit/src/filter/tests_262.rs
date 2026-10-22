#[cfg(test)]
mod tests {
    use log::{Level, LevelFilter};

    use super::{enabled, Builder, Directive, Filter};

    fn make_logger_filter(dirs: Vec<Directive>) -> Filter {
        let mut logger = Builder::new().build();
        logger.directives = dirs;
        logger
    }

    #[test]
    fn filter_info() {
        let logger = Builder::new().filter(None, LevelFilter::Info).build();
        assert!(enabled(&logger.directives, Level::Info, "crate1"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate1"));
    }

    #[test]
    fn filter_beginning_longest_match() {
        let logger = Builder::new()
            .filter(Some("crate2"), LevelFilter::Info)
            .filter(Some("crate2::mod"), LevelFilter::Debug)
            .filter(Some("crate1::mod1"), LevelFilter::Warn)
            .build();
        assert!(enabled(&logger.directives, Level::Debug, "crate2::mod1"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
    }

    // Some of our tests are only correct or complete when they cover the full
    // universe of variants for log::Level. In the unlikely event that a new
    // variant is added in the future, this test will detect the scenario and
    // alert us to the need to review and update the tests. In such a
    // situation, this test will fail to compile, and the error message will
    // look something like this:
    //
    //     error[E0004]: non-exhaustive patterns: `NewVariant` not covered
    //        --> src/filter/mod.rs:413:15
    //         |
    //     413 |         match level_universe {
    //         |               ^^^^^^^^^^^^^^ pattern `NewVariant` not covered
    #[test]
    fn ensure_tests_cover_level_universe() {
        let level_universe: Level = Level::Trace; // use of trace variant is arbitrary
        match level_universe {
            Level::Error | Level::Warn | Level::Info | Level::Debug | Level::Trace => (),
        }
    }

    #[test]
    fn parse_default() {
        let logger = Builder::new()
            .try_parse("info,crate1::mod1=warn")
            .unwrap()
            .build();
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }

    #[test]
    fn parse_default_bare_level_off_lc() {
        let logger = Builder::new().try_parse("off").unwrap().build();
        assert!(!enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_off_uc() {
        let logger = Builder::new().try_parse("OFF").unwrap().build();
        assert!(!enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_error_lc() {
        let logger = Builder::new().try_parse("error").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_error_uc() {
        let logger = Builder::new().try_parse("ERROR").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(!enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_warn_lc() {
        let logger = Builder::new().try_parse("warn").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_warn_uc() {
        let logger = Builder::new().try_parse("WARN").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(!enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_info_lc() {
        let logger = Builder::new().try_parse("info").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_info_uc() {
        let logger = Builder::new().try_parse("INFO").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(!enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_debug_lc() {
        let logger = Builder::new().try_parse("debug").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_debug_uc() {
        let logger = Builder::new().try_parse("DEBUG").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(!enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_trace_lc() {
        let logger = Builder::new().try_parse("trace").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(enabled(&logger.directives, Level::Trace, ""));
    }

    #[test]
    fn parse_default_bare_level_trace_uc() {
        let logger = Builder::new().try_parse("TRACE").unwrap().build();
        assert!(enabled(&logger.directives, Level::Error, ""));
        assert!(enabled(&logger.directives, Level::Warn, ""));
        assert!(enabled(&logger.directives, Level::Info, ""));
        assert!(enabled(&logger.directives, Level::Debug, ""));
        assert!(enabled(&logger.directives, Level::Trace, ""));
    }

    // In practice, the desired log level is typically specified by a token
    // that is either all lowercase (e.g., 'trace') or all uppercase (.e.g,
    // 'TRACE'), but this tests serves as a reminder that
    // log::Level::from_str() ignores all case variants.
    #[test]
    fn parse_default_bare_level_debug_mixed() {
        {
            let logger = Builder::new().try_parse("Debug").unwrap().build();
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
        {
            let logger = Builder::new().try_parse("debuG").unwrap().build();
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
        {
            let logger = Builder::new().try_parse("deBug").unwrap().build();
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
        {
            let logger = Builder::new().try_parse("DeBuG").unwrap().build(); // LaTeX flavor!
            assert!(enabled(&logger.directives, Level::Error, ""));
            assert!(enabled(&logger.directives, Level::Warn, ""));
            assert!(enabled(&logger.directives, Level::Info, ""));
            assert!(enabled(&logger.directives, Level::Debug, ""));
            assert!(!enabled(&logger.directives, Level::Trace, ""));
        }
    }

    #[test]
    fn try_parse_valid_filter() {
        let logger = Builder::new()
            .try_parse("info,crate1::mod1=warn")
            .expect("valid filter returned error")
            .build();
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }

    #[test]
    fn try_parse_invalid_filter() {
        let error = Builder::new().try_parse("info,crate1=invalid").unwrap_err();
        assert_eq!(
            (error).to_string(),
            "error parsing logger filter: invalid logging spec 'invalid'"
        );
    }

    #[test]
    fn match_full_path() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(!enabled(&logger.directives, Level::Info, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
    }

    #[test]
    fn no_match() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(!enabled(&logger.directives, Level::Warn, "crate3"));
    }

    #[test]
    fn match_beginning() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod1"));
    }

    #[test]
    fn match_beginning_longest_match() {
        let logger = make_logger_filter(vec![
            Directive {
                name: Some("crate2".to_owned()),
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate2::mod".to_owned()),
                level: LevelFilter::Debug,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Debug, "crate2::mod1"));
        assert!(!enabled(&logger.directives, Level::Debug, "crate2"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger_filter(vec![
            Directive {
                name: None,
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Warn,
            },
        ]);
        assert!(enabled(&logger.directives, Level::Warn, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger_filter(vec![
            Directive {
                name: None,
                level: LevelFilter::Info,
            },
            Directive {
                name: Some("crate1::mod1".to_owned()),
                level: LevelFilter::Off,
            },
        ]);
        assert!(!enabled(&logger.directives, Level::Error, "crate1::mod1"));
        assert!(enabled(&logger.directives, Level::Info, "crate2::mod2"));
    }
}
