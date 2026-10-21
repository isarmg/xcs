use super::*;
#[test]
fn portable_names() {
    for s in [
        "报告-2026.docx",
        ".gitignore",
        "~$报告.docx",
        "报告（终稿）.docx",
        "项目、预算.xlsx",
    ] {
        validate_name(s).unwrap();
    }
    for s in [
        "a:b.txt",
        "CON",
        "nul.txt",
        "COM¹.txt",
        "LPT².txt",
        "CON .txt",
        " bad",
        "bad.",
        ".",
        "..",
        "a\u{200d}b",
        "🚀",
    ] {
        assert!(validate_name(s).is_err(), "{s}");
    }
}
#[test]
fn normalized_and_folded() {
    assert_eq!(lookup_key("Straße.txt"), "strasse.txt");
    assert_eq!(lookup_key("Σ"), lookup_key("ς"));
    assert_eq!(lookup_key("e\u{301}"), lookup_key("é"));
    assert_eq!(
        validate_name("e\u{301}").unwrap_err().code(),
        "NAME_NOT_NORMALIZED"
    );
    assert_eq!(unicode::UNICODE_VERSION, (16, 0, 0));
}
#[test]
fn path_budgets() {
    assert!(validate_path("../escape").is_err());
    assert!(validate_path("a//b").is_err());
    assert!(validate_path("/root").is_err());
    assert!(validate_path(&vec!["a"; 33].join("/")).is_err());
}
