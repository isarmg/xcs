#[cfg(test)]
mod tests {
    use super::{analyze_pattern, PatternOptimization};
    use test_case::test_case;

    #[test_case(r"^\S*$", Some(PatternOptimization::NoWhitespace) ; "no whitespace sentinel")]
    #[test_case(
        r"^(get|put|post)$",
        Some(PatternOptimization::Alternation(vec!["get".into(), "post".into(), "put".into()])) ;
        "sorted alternation"
    )]
    #[test_case(r"^(a|b|c^)$", None ; "invalid char in alternative")]
    #[test_case(r"^(x-foo|x-bar)$", Some(PatternOptimization::Alternation(vec!["x-bar".into(), "x-foo".into()])) ; "alternation with dash")]
    #[test_case(r"^(single)$", Some(PatternOptimization::Alternation(vec!["single".into()])) ; "single alternative")]
    #[allow(clippy::needless_pass_by_value)]
    fn test_analyze_pattern_new(pattern: &str, expected: Option<PatternOptimization>) {
        assert_eq!(analyze_pattern(pattern), expected);
    }
}
