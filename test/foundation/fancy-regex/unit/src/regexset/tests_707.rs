#[cfg(test)]
mod tests {
    use super::{
        RegexSet, RegexSetOptions, DEFAULT_META_HYBRID_CACHE_CAPACITY, DEFAULT_META_NFA_SIZE_LIMIT,
        DEFAULT_OVERLAPPING_DFA_CACHE_CAPACITY,
    };
    use crate::{Error, RegexInput, RegexOptionsBuilder, RuntimeError};

    #[test]
    fn regex_set_options_defaults_to_larger_regex_automata_limits() {
        let options = RegexSetOptions::default();

        assert_eq!(
            options.meta_nfa_size_limit,
            Some(DEFAULT_META_NFA_SIZE_LIMIT)
        );
        assert_eq!(
            options.meta_hybrid_cache_capacity,
            DEFAULT_META_HYBRID_CACHE_CAPACITY
        );
        assert_eq!(
            options.overlapping_dfa_cache_capacity,
            DEFAULT_OVERLAPPING_DFA_CACHE_CAPACITY
        );
        assert!(options.overlapping_dfa_skip_cache_capacity_check);
    }

    #[test]
    fn regex_set_options_setters_update_regex_automata_limits() {
        let options = RegexSetOptions::new()
            .delegate_size_limit(11)
            .delegate_dfa_size_limit(22)
            .meta_nfa_size_limit(None)
            .meta_hybrid_cache_capacity(33)
            .overlapping_dfa_cache_capacity(44)
            .overlapping_dfa_skip_cache_capacity_check(false);

        assert_eq!(options.delegate_size_limit, Some(11));
        assert_eq!(options.delegate_dfa_size_limit, Some(22));
        assert_eq!(options.meta_nfa_size_limit, None);
        assert_eq!(options.meta_hybrid_cache_capacity, 33);
        assert_eq!(options.overlapping_dfa_cache_capacity, 44);
        assert!(!options.overlapping_dfa_skip_cache_capacity_check);
    }

    #[test]
    fn find_input_returns_all_matches_at_earliest_position_in_pattern_order() {
        let set = RegexSet::new(&[r"\d+", r"\w+", r"(?<=\$)\d+\.\d+"]).unwrap();
        let mut matches = set.find_input(RegexInput::new("$29.99")).unwrap().unwrap();

        let first = matches.next().unwrap().unwrap();
        assert_eq!(0, first.pattern());
        assert_eq!(1, first.start());
        assert_eq!(3, first.end());
        assert_eq!("29", first.as_str());

        let second = matches.next().unwrap().unwrap();
        assert_eq!(1, second.pattern());
        assert_eq!(1, second.start());
        assert_eq!(3, second.end());
        assert_eq!("29", second.as_str());

        let third = matches.next().unwrap().unwrap();
        assert_eq!(2, third.pattern());
        assert_eq!(1, third.start());
        assert_eq!(6, third.end());
        assert_eq!("29.99", third.as_str());

        assert!(matches.next().is_none());
    }

    #[test]
    fn find_input_skips_false_positive_candidate_positions() {
        let set = RegexSet::new(&[r"(?<=foo)bar"]).unwrap();
        let mut matches = set
            .find_input(RegexInput::new("barfoobar"))
            .unwrap()
            .unwrap();

        let only = matches.next().unwrap().unwrap();
        assert_eq!(0, only.pattern());
        assert_eq!(6, only.start());
        assert_eq!(9, only.end());
        assert_eq!("bar", only.as_str());
        assert!(matches.next().is_none());
    }

    #[test]
    fn find_input_returns_none_when_input_is_done() {
        let set = RegexSet::new(&[r"."]).unwrap();

        assert!(set
            .find_input(RegexInput::new("a").from_pos(2))
            .unwrap()
            .is_none());
    }

    #[test]
    fn find_input_returns_none_when_input_is_anchored_and_match_not_at_start_position() {
        let set = RegexSet::new(&[r"b"]).unwrap();

        assert!(set
            .find_input(RegexInput::new("ab").from_pos(0).anchored(true))
            .unwrap()
            .is_none());
    }

    #[test]
    fn find_input_returns_match_when_input_is_anchored_and_match_at_start_position() {
        let set = RegexSet::new(&[r"b"]).unwrap();

        let mut matches = set
            .find_input(RegexInput::new("ab").from_pos(1).anchored(true))
            .unwrap()
            .unwrap();

        let only = matches.next().unwrap().unwrap();
        assert_eq!(0, only.pattern());
        assert_eq!(1, only.start());
        assert_eq!(2, only.end());
        assert_eq!("b", only.as_str());
        assert!(matches.next().is_none());
    }

    #[test]
    fn find_input_defers_later_pattern_evaluation_until_iteration() {
        let mut options_builder = RegexOptionsBuilder::new();
        options_builder.backtrack_limit(0);
        let set = RegexSet::new_with_options(&[r"a", r"(?:(a|aa)+)\1"], &options_builder).unwrap();

        let mut matches = set.find_input(RegexInput::new("aa")).unwrap().unwrap();

        let first = matches.next().unwrap().unwrap();
        assert_eq!(0, first.pattern());
        assert_eq!(0, first.start());
        assert_eq!(1, first.end());

        let second = matches.next().unwrap();
        assert!(matches!(
            second,
            Err(Error::RuntimeError(RuntimeError::BacktrackLimitExceeded))
        ));
    }

    #[test]
    fn find_input_picks_earliest_start_position_before_iterating_pattern_order() {
        let mut options_builder = RegexOptionsBuilder::new();
        options_builder.multi_line(true);
        let set = RegexSet::new_with_options(
            &[
                r"//.*$",
                r#""(?:[^"\\]|\\.)*""#,
                r"\b(fn|let|mut|if|else)\b",
                r"\b[0-9]+\b",
                r"[a-zA-Z_][a-zA-Z0-9_]*",
            ],
            &options_builder,
        )
        .unwrap();

        let mut matches = set
            .find_input(RegexInput::new(
                "let x = 42; // a comment\nlet s = \"hello world\";",
            ))
            .unwrap()
            .unwrap();

        let first = matches.next().unwrap().unwrap();
        assert_eq!(2, first.pattern());
        assert_eq!(0, first.start());
        assert_eq!(3, first.end());
        assert_eq!("let", first.as_str());
    }

    #[test]
    fn find_input_yields_each_pattern_at_match_start_once() {
        let set = RegexSet::new(&[r"a+", r"a"]).unwrap();
        let mut matches = set.find_input(RegexInput::new("aaa")).unwrap().unwrap();

        let first = matches.next().unwrap().unwrap();
        assert_eq!(0, first.pattern());
        assert_eq!(0, first.start());
        assert_eq!(3, first.end());
        assert_eq!("aaa", first.as_str());

        let second = matches.next().unwrap().unwrap();
        assert_eq!(1, second.pattern());
        assert_eq!(0, second.start());
        assert_eq!(1, second.end());
        assert_eq!("a", second.as_str());

        assert!(matches.next().is_none());
    }

    #[test]
    fn test_no_captures_returns_group_0() {
        let set = RegexSet::new(&[r"\w+"]).unwrap();
        let mut matches = set.find_input(RegexInput::new("abc")).unwrap().unwrap();

        let only = matches.next().unwrap().unwrap();
        assert_eq!(0, only.pattern());
        assert_eq!(1, only.captures().len());
        assert_eq!("abc", only.as_str());
    }

    #[test]
    fn word_boundary_matches_correctly_with_unicode_text() {
        // \bbar\b uses a unicode word boundary; the DFA may encounter quit bytes
        // for multi-byte UTF-8 characters such as 'é' (0xC3 0xA9). The RegexSet
        // must not panic and must still return the correct match.
        let set = RegexSet::new([r"foo", r"\bbar\b"]).unwrap();
        let mut matches = set
            .find_input(RegexInput::new("fooé bar"))
            .unwrap()
            .unwrap();

        // "foo" is the earliest match (position 0)
        let first = matches.next().unwrap().unwrap();
        assert_eq!(0, first.pattern());
        assert_eq!("foo", first.as_str());
        assert!(matches.next().is_none());

        // "bar" is matched later in the string (after the unicode character)
        let mut matches2 = set
            .find_input(RegexInput::new("fooé bar").from_pos(first.end()))
            .unwrap()
            .unwrap();
        let bar_match = matches2.next().unwrap().unwrap();
        assert_eq!(1, bar_match.pattern());
        assert_eq!("bar", bar_match.as_str());
        assert!(matches2.next().is_none());
    }

    #[test]
    fn find_input_continue_from_prev_match_inside_negative_lookbehind() {
        let options = &mut RegexOptionsBuilder::new();
        let options = options.allow_input_assertion_overrides(true);
        let set = RegexSet::new_with_options([r"(?<!\G)b"], &options).unwrap();
        let mut matches = set
            .find_input(RegexInput::new("ab").continue_from_previous_match_end(false))
            .unwrap()
            .unwrap();

        let first = matches.next().unwrap().unwrap();
        assert_eq!(0, first.pattern());
        assert_eq!("b", first.as_str());
        assert!(matches.next().is_none());
    }

    #[test]
    fn continue_from_prev_match_works_as_expected_when_match_is_not_at_search_start() {
        use crate::Arc;
        use crate::RegexBuilder;

        let (pat, hay) = (r"\Gx", "yx");

        // `\G` holds only at the search start (index 0); 'x' is at 1 -> no match.
        let re = RegexBuilder::new(pat).build().unwrap();
        let single = re
            .find_input(RegexInput::new(hay).from_pos(0))
            .unwrap()
            .map(|m| (m.start(), m.end()));

        // Same regex, same input, via a RegexSet:
        let set = RegexSet::from_regexes([Arc::new(re)], Default::default()).unwrap();
        let via_set = set
            .find_input(RegexInput::new(hay).from_pos(0))
            .unwrap()
            .and_then(|mut it| it.next())
            .map(|m| {
                let m = m.unwrap();
                (m.start(), m.end())
            });

        assert_eq!(
            single, via_set,
            "RegexSet should behave the same as a standalone regex"
        );
        assert_eq!(single, None);
    }

    #[test]
    fn continue_from_prev_match_works_as_expected_when_match_is_at_search_start() {
        use crate::Arc;
        use crate::RegexBuilder;

        let (pat, hay) = (r"\Gx", "yx");

        let re = RegexBuilder::new(pat).build().unwrap();
        let single = re
            .find_input(RegexInput::new(hay).from_pos(1))
            .unwrap()
            .map(|m| (m.start(), m.end()));

        // Same regex, same input, via a RegexSet:
        let set = RegexSet::from_regexes([Arc::new(re)], Default::default()).unwrap();
        let via_set = set
            .find_input(RegexInput::new(hay).from_pos(1))
            .unwrap()
            .and_then(|mut it| it.next())
            .map(|m| {
                let m = m.unwrap();
                (m.start(), m.end())
            });

        assert_eq!(
            single, via_set,
            "RegexSet should behave the same as a standalone regex"
        );
        assert_eq!(single, Some((1, 2)));
    }
}
