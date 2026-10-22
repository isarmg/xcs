#[test]
fn streaming_matcher_uses_shared_dfa() {
    let pattern = tracing::pattern::Pattern::new("hel+o w[orl]{3}d").unwrap();
    assert!(pattern.display_matches(&"hello world"));
    assert!(!pattern.display_matches(&"goodbye world"));
}

#[test]
fn fancy_backreference_and_lookbehind() {
    let pattern = fancy_regex::Regex::new(r"(?<=prefix:)([a-z]+)-\1").unwrap();
    assert!(pattern.is_match("prefix:abc-abc").unwrap());
    assert!(!pattern.is_match("prefix:abc-def").unwrap());
}

#[test]
fn schema_parser_translation_uses_shared_syntax() {
    let translated = schema_validation::patterns::to_rust_regex(r"\cA").unwrap();
    let pattern = regex::Regex::new(&translated).unwrap();
    assert!(pattern.is_match("\x01"));
    assert!(!pattern.is_match("A"));
}

#[test]
fn tracing_filter_keeps_regex_field_support() {
    let filter = tracing::collector::EnvFilter::try_new("info,[span{value=abc.*}]=trace");
    assert!(filter.is_ok());
}
