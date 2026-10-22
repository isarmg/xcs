#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use super::{parse_spec, ParseResult};

    #[test]
    fn parse_spec_valid() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug");

        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_owned()));
        assert_eq!(dirs[1].level, LevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_owned()));
        assert_eq!(dirs[2].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_invalid_crate() {
        // test parse_spec with multiple = in specification
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=warn=info,crate2=debug");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_eq!(
            (&errors[0]).to_string(),
            "invalid logging spec 'crate1::mod1=warn=info'"
        );
    }

    #[test]
    fn parse_spec_invalid_level() {
        // test parse_spec with 'noNumber' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=noNumber,crate2=debug");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_eq!((&errors[0]).to_string(), "invalid logging spec 'noNumber'");
    }

    #[test]
    fn parse_spec_string_level() {
        // test parse_spec with 'warn' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=wrong,crate2=warn");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_eq!((&errors[0]).to_string(), "invalid logging spec 'wrong'");
    }

    #[test]
    fn parse_spec_empty_level() {
        // test parse_spec with '' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=wrong,crate2=");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::max());
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_eq!((&errors[0]).to_string(), "invalid logging spec 'wrong'");
    }

    #[test]
    fn parse_spec_empty_level_isolated() {
        // test parse_spec with "" as log level (and the entire spec str)
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec(""); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated() {
        // test parse_spec with a white-space-only string specified as the log
        // level (and the entire spec str)
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("     "); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated_comma_only() {
        // The spec should contain zero or more comma-separated string slices,
        // so a comma-only string should be interpreted as two empty strings
        // (which should both be treated as invalid, so ignored).
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec(","); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated_comma_blank() {
        // The spec should contain zero or more comma-separated string slices,
        // so this bogus spec should be interpreted as containing one empty
        // string and one blank string. Both should both be treated as
        // invalid, so ignored.
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec(",     "); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_blank_level_isolated_blank_comma() {
        // The spec should contain zero or more comma-separated string slices,
        // so this bogus spec should be interpreted as containing one blank
        // string and one empty string. Both should both be treated as
        // invalid, so ignored.
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("     ,"); // should be ignored
        assert_eq!(dirs.len(), 0);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global() {
        // test parse_spec with no crate
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert_eq!(dirs[1].name, Some("crate2".to_owned()));
        assert_eq!(dirs[1].level, LevelFilter::Debug);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global_bare_warn_lc() {
        // test parse_spec with no crate, in isolation, all lowercase
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global_bare_warn_uc() {
        // test parse_spec with no crate, in isolation, all uppercase
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("WARN");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_global_bare_warn_mixed() {
        // test parse_spec with no crate, in isolation, mixed case
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("wArN");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_valid_filter() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug/abc");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_owned()));
        assert_eq!(dirs[1].level, LevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_owned()));
        assert_eq!(dirs[2].level, LevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "abc");
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_invalid_crate_filter() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=error=warn,crate2=debug/a.c");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "a.c");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            (&errors[0]).to_string(),
            "invalid logging spec 'crate1::mod1=error=warn'"
        );
    }

    #[test]
    fn parse_spec_empty_with_filter() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1/a*c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate1".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::max());
        assert!(filter.is_some() && filter.unwrap().to_string() == "a*c");
        assert!(errors.is_empty());
    }

    #[test]
    fn parse_spec_with_multiple_filters() {
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("debug/abc/a.c");
        assert!(dirs.is_empty());
        assert!(filter.is_none());

        assert_eq!(errors.len(), 1);
        assert_eq!(
            (&errors[0]).to_string(),
            "invalid logging spec 'debug/abc/a.c' (too many '/'s)"
        );
    }

    #[test]
    fn parse_spec_multiple_invalid_crates() {
        // test parse_spec with multiple = in specification
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=warn=info,crate2=debug,crate3=error=error");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 2);
        assert_eq!(
            (&errors[0]).to_string(),
            "invalid logging spec 'crate1::mod1=warn=info'"
        );
        assert_eq!(
            (&errors[1]).to_string(),
            "invalid logging spec 'crate3=error=error'"
        );
    }

    #[test]
    fn parse_spec_multiple_invalid_levels() {
        // test parse_spec with 'noNumber' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=noNumber,crate2=debug,crate3=invalid");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 2);
        assert_eq!((&errors[0]).to_string(), "invalid logging spec 'noNumber'");
        assert_eq!((&errors[1]).to_string(), "invalid logging spec 'invalid'");
    }

    #[test]
    fn parse_spec_invalid_crate_and_level() {
        // test parse_spec with 'noNumber' as log level
        let ParseResult {
            directives: dirs,
            filter,
            errors,
        } = parse_spec("crate1::mod1=debug=info,crate2=debug,crate3=invalid");

        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_owned()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());

        assert_eq!(errors.len(), 2);
        assert_eq!(
            (&errors[0]).to_string(),
            "invalid logging spec 'crate1::mod1=debug=info'"
        );
        assert_eq!((&errors[1]).to_string(), "invalid logging spec 'invalid'");
    }

    #[test]
    fn parse_error_message_single_error() {
        let error = parse_spec("crate1::mod1=debug=info,crate2=debug")
            .ok()
            .unwrap_err();
        assert_eq!(
            (error).to_string(),
            "error parsing logger filter: invalid logging spec 'crate1::mod1=debug=info'"
        );
    }

    #[test]
    fn parse_error_message_multiple_errors() {
        let error = parse_spec("crate1::mod1=debug=info,crate2=debug,crate3=invalid")
            .ok()
            .unwrap_err();
        assert_eq!(
            (error).to_string(),
            "error parsing logger filter: invalid logging spec 'crate1::mod1=debug=info'"
        );
    }
}
