#[cfg(test)]
mod tests {
    use alloc::{format, string::String, vec::Vec};

    use crate::{meta::regex::Config, util::syntax};

    use super::*;

    fn teddy_probably_available() -> bool {
        cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
    }

    #[track_caller]
    fn strategy_with(patterns: &[&str], config: Config) -> Arc<dyn Strategy> {
        let hirs = syntax::parse_many(patterns).unwrap();
        let hirs: Vec<&Hir> = hirs.iter().collect();
        let info = RegexInfo::new(config, &hirs);
        new(&info, &hirs).unwrap()
    }

    #[track_caller]
    fn assert_strategy(name: &'static str, patterns: &[&str]) {
        assert_strategy_with(name, patterns, Config::new());
    }

    #[track_caller]
    fn assert_strategy_with(
        name: &'static str,
        patterns: &[&str],
        config: Config,
    ) {
        let strategy = strategy_with(patterns, config);
        assert_eq!(name, strategy.name().as_ref());
    }

    fn literal_alternation(count: usize) -> String {
        let mut pattern = String::new();
        for i in 0..count {
            if i > 0 {
                pattern.push('|');
            }
            pattern.push_str(&format!("needle{i:05}"));
        }
        pattern
    }

    #[test]
    fn pre_from_prefixes_accepts_exact_literal() {
        assert_strategy("prefilter memchr", &["a"]);
        assert_strategy("prefilter memchr2", &["a|b"]);
        assert_strategy("prefilter memchr3", &["a|b|c"]);
        assert_strategy("prefilter memmem", &["Sherlock"]);
        if teddy_probably_available() {
            assert_strategy(
                "prefilter teddy",
                &["Samwise|Gandalf|Holmes|Watson"],
            );
        }
    }

    #[test]
    fn pre_from_prefixes_rejects_inexact_literal() {
        assert_strategy("core", &["a+"]);
    }

    #[test]
    fn pre_from_prefixes_rejects_empty_literal() {
        assert_strategy("core", &[""]);
    }

    #[test]
    fn pre_from_prefixes_rejects_multiple_patterns() {
        assert_strategy("core", &["a", "b"]);
        assert_strategy("core", &["a|b", "c|d"]);
    }

    #[test]
    fn pre_from_prefixes_rejects_captures() {
        assert_strategy("core", &["(a)"]);
        // ... but not when it's a non-capture.
        assert_strategy("prefilter memchr", &["(?:a)"]);
        // ... or when there is a capture, but it's optimized out.
        assert_strategy("prefilter memchr", &["(){0}a"]);
    }

    #[test]
    fn pre_from_prefixes_rejects_look_around() {
        assert_strategy("core", &[r"a\b"]);
    }

    #[test]
    fn pre_from_prefixes_rejects_non_leftmost_first() {
        assert_strategy_with(
            "core",
            &["a"],
            Config::new().match_kind(MatchKind::All),
        );
    }

    #[test]
    fn pre_from_alternation_literals_accepts_large_alternation() {
        let pattern = literal_alternation(3_000);
        assert_strategy("prefilter aho-corasick", &[&pattern]);
    }

    #[test]
    fn pre_from_alternation_literals_rejects_small_alternation() {
        let pattern = literal_alternation(2_999);
        assert_strategy("core", &[&pattern]);
    }

    #[test]
    fn pre_from_alternation_literals_rejects_captures() {
        let pattern = format!("({})", literal_alternation(3_000));
        assert_strategy("core", &[&pattern]);
    }

    #[test]
    fn pre_from_alternation_literals_rejects_look_around() {
        let pattern = format!(r"(?:{})\b", literal_alternation(3_000));
        assert_strategy("core", &[&pattern]);
    }

    #[test]
    fn pre_from_alternation_literals_rejects_non_leftmost_first() {
        let pattern = literal_alternation(3_000);
        assert_strategy_with(
            "core",
            &[&pattern],
            Config::new().match_kind(MatchKind::All),
        );
    }

    #[test]
    fn pre_from_alternation_literals_rejects_multiple_patterns() {
        let pattern1 = literal_alternation(3_000);
        let pattern2 = literal_alternation(3_000);
        assert_strategy("core", &[&pattern1, &pattern2]);
    }

    #[test]
    fn core_selected_when_no_special_strategy_applies() {
        assert_strategy("core", &["[a-z]+"]);
    }

    #[test]
    fn core_selected_when_auto_prefilter_is_disabled() {
        assert_strategy_with(
            "core",
            &[r"\w+Holmes"],
            Config::new().auto_prefilter(false),
        );
    }

    #[test]
    fn core_selected_when_reverse_engines_are_disabled() {
        assert_strategy_with(
            "core",
            &[r"\w+Holmes"],
            Config::new().dfa(false).hybrid(false),
        );
    }

    #[test]
    fn reverse_anchored_accepts_end_anchored() {
        assert_strategy("reverse anchored", &[r"\w+Holmes$"]);
    }

    #[test]
    fn reverse_anchored_accepts_multiple_end_anchored_patterns() {
        assert_strategy("reverse anchored", &[r"\w+Holmes$", r"\w+Watson$"]);
    }

    #[test]
    fn reverse_anchored_accepts_hybrid_without_full_dfa() {
        assert_strategy_with(
            "reverse anchored",
            &[r"\w+Holmes$"],
            Config::new().dfa(false),
        );
    }

    #[test]
    fn reverse_anchored_rejects_start_and_end_anchored() {
        assert_strategy("core", &[r"^\w+Holmes$"]);
    }

    #[test]
    fn reverse_anchored_rejects_without_reverse_engine() {
        assert_strategy_with(
            "core",
            &[r"\w+Holmes$"],
            Config::new().dfa(false).hybrid(false),
        );
    }

    #[test]
    fn reverse_suffix_accepts_prefix_without_internal_suffix() {
        assert_strategy("reverse suffix", &[r"\d+XYZ"]);
        assert_strategy("reverse suffix", &[r"[a-q][^u-z]{13}x"]);
    }

    #[test]
    fn reverse_suffix_rejects_internal_suffix_with_word_boundary() {
        assert_strategy("core", &[r"\b\w+nn\b"]);
    }

    #[test]
    fn reverse_suffix_accepts_disjoint_class_separator() {
        assert_strategy("reverse suffix", &[r"\w+\s+Holmes"]);
        assert_strategy("reverse suffix", &[r"\w+\d*\s+Holmes"]);
        assert_strategy("reverse suffix", &[r"(?:\w+|[.-]+)+\s+Holmes"]);
        assert_strategy("reverse suffix", &[r"\b\w+\s+Holmes"]);
        assert_strategy("reverse suffix", &[r"\w+\sHolmes"]);
        assert_strategy("reverse suffix", &[r"[a-z]+[0-9]+Holmes"]);
        assert_strategy("reverse suffix", &[r"(\w+)(\s+)Holmes"]);
        assert_strategy("reverse suffix", &[r"(?-u:[A-Za-z]+[0-9]+Holmes)"]);
    }

    #[test]
    fn reverse_suffix_class_separator_is_conservative() {
        assert_strategy("core", &[r"\w+\w+Holmes"]);
        assert_strategy("core", &[r"\w+\s*Holmes"]);
        assert_strategy("core", &[r"[a-z]+[0-9a]+xyz"]);
        assert_strategy("core", &[r"[a-z]+[0-9]+a1"]);
        assert_strategy("core", &[r"(?:.abc)?[a]+[b]+c"]);
    }

    #[test]
    fn reverse_suffix_rejects_safe_nfa_overlap() {
        assert_strategy("reverse inner", &[r"(a|aa)b"]);
    }

    #[test]
    fn reverse_suffix_accepts_fixed_length_prefix() {
        assert_strategy("reverse suffix", &[r"[A-Z][0-9]XYZ"]);
    }

    #[test]
    fn reverse_suffix_rejects_multiple_patterns_with_common_suffix() {
        assert_strategy("core", &[r"\d+XYZ", r"\w+XYZ"]);
    }

    #[test]
    fn reverse_suffix_rejects_auto_prefilter_disabled() {
        assert_strategy_with(
            "core",
            &[r"\d+XYZ"],
            Config::new().auto_prefilter(false),
        );
    }

    #[test]
    fn reverse_suffix_rejects_non_leftmost_first() {
        assert_strategy_with(
            "core",
            &[r"\d+XYZ"],
            Config::new().match_kind(MatchKind::All),
        );
    }

    #[test]
    fn reverse_suffix_rejects_anchored_start() {
        assert_strategy("core", &[r"^\d+XYZ"]);
    }

    #[test]
    fn reverse_suffix_rejects_variable_length_prefix() {
        assert_strategy("core", &[r"(?:[A-Za-z]ab)?b"]);
    }

    #[test]
    fn reverse_suffix_rejects_without_reverse_engine() {
        assert_strategy_with(
            "core",
            &[r"\d+XYZ"],
            Config::new().dfa(false).hybrid(false),
        );
    }

    #[test]
    fn reverse_suffix_rejects_existing_fast_prefilter() {
        assert_strategy("core", &[r"abc\w+XYZ"]);
    }

    #[test]
    fn reverse_suffix_rejects_no_literal_suffix() {
        assert_strategy("core", &[r"[a-z]+"]);
    }

    #[test]
    fn reverse_suffix_rejects_multiple_patterns_without_common_suffix() {
        assert_strategy("core", &[r"\d+XYZ", r"\w+ABC"]);
    }

    #[test]
    fn reverse_suffix_rejects_multiple_patterns_when_first_looks_safe() {
        assert_strategy("core", &[r"\d+b", r".bb|b"]);
        assert_strategy("core", &[r"\d+b", r"ab"]);
    }

    #[test]
    fn reverse_suffix_rejects_unsafe_overlap() {
        assert_strategy("core", &[r".abb|b"]);
        assert_strategy("core", &[r".bb|b"]);
    }

    #[test]
    fn reverse_suffix_rejects_fully_absorbed_literal() {
        assert_strategy("core", &[r"(?:[a-wyz]{3}|[a-wyz]).b"]);
    }

    #[test]
    fn reverse_suffix_rejects_disjoint_trailing_component() {
        assert_strategy("core", &[r"(?:[ac-z]{2}b[ac-z])?[ac-z]b"]);
    }

    #[test]
    fn reverse_suffix_rejects_internal_candidate_before_absorber() {
        assert_strategy("core", &[r"(?:[a-z]cb|c)[a-z]*b"]);
    }

    #[test]
    fn reverse_inner_accepts_disjoint_inner_literal() {
        assert_strategy("reverse inner", &[r"\w+@\w+"]);
        assert_strategy(
            "reverse inner",
            &[r"(?P<email>[.\pL]+@(?P<domain>[.\pL]+))"],
        );
    }

    #[test]
    fn reverse_inner_accepts_fixed_length_prefix() {
        assert_strategy("reverse inner", &[r"[A-Z][0-9]@(foo|bar)"]);
    }

    #[test]
    fn reverse_inner_accepts_disjoint_class_separator() {
        assert_strategy("reverse inner", &[r"\w+\s+Holmes\s+\w+"]);
        assert_strategy("reverse inner", &[r"\w+\d*\s+Holmes\s+\w+"]);
        assert_strategy("reverse inner", &[r"\b\w+\s+Holmes\s+\w+"]);
        if teddy_probably_available() {
            assert_strategy(
                "reverse inner",
                &[r"\w+\s+(?:Holmes|Watson)\s+\w+"],
            );
        }
    }

    #[test]
    fn reverse_inner_accepts_leading_disjoint_class_separator() {
        assert_strategy("reverse inner", &[r"\s[A-Za-z]{0,12}ing\s"]);
        assert_strategy("reverse inner", &[r"\s[A-Za-z]*ing\s"]);
        assert_strategy("reverse inner", &[r"\b\s[A-Za-z]{0,12}ing\s"]);
        assert_strategy("reverse inner", &[r"(\s)([A-Za-z]{0,12})ing\s"]);
    }

    #[test]
    fn reverse_inner_leading_class_separator_is_conservative() {
        assert_strategy("core", &[r"\s[\sA-Za-z]{0,12}ing\s"]);
        assert_strategy("core", &[r"\s*[A-Za-z]{0,12}ing\s"]);
    }

    #[test]
    fn reverse_inner_rejects_auto_prefilter_disabled() {
        assert_strategy_with(
            "core",
            &[r"\w+@\w+"],
            Config::new().auto_prefilter(false),
        );
    }

    #[test]
    fn reverse_inner_rejects_non_leftmost_first() {
        assert_strategy_with(
            "core",
            &[r"\w+@\w+"],
            Config::new().match_kind(MatchKind::All),
        );
    }

    #[test]
    fn reverse_inner_rejects_anchored_start() {
        assert_strategy("core", &[r"^\w+@\w+"]);
    }

    #[test]
    fn reverse_inner_rejects_without_reverse_engine() {
        assert_strategy_with(
            "core",
            &[r"\w+@\w+"],
            Config::new().dfa(false).hybrid(false),
        );
    }

    #[test]
    fn reverse_inner_rejects_existing_fast_prefilter() {
        assert_strategy("core", &[r"abc\w+@\w+"]);
    }

    #[test]
    fn reverse_inner_rejects_multiple_patterns() {
        assert_strategy("core", &[r"\w+@\w+", r"\w+#\w+"]);
    }

    #[test]
    fn reverse_inner_rejects_no_top_level_concat() {
        assert_strategy("core", &[r"\w+"]);
    }

    #[test]
    fn reverse_inner_rejects_unsafe_overlap() {
        assert_strategy("core", &[r"a.*@\w+"]);
    }

    #[test]
    fn reverse_inner_rejects_literal_crossing_prefix_boundary() {
        assert_strategy("core", &[r"(?:[ac-z]{3}|[ac-z])(?:aba|baa)"]);
    }

    #[test]
    fn reverse_inner_rejects_internal_separator_alignment() {
        assert_strategy("core", &[r"(?:.abc)?[a]+[b]+c.*"]);
    }

    #[test]
    fn rebar_benchmarks() {
        assert_strategy("reverse suffix", &[r"[a-z]shing"]);
        assert_strategy("reverse suffix", &[r"[a-q][^u-z]{23}x"]);
    }
}
