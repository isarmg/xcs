#[test]
fn test_writeable() {
    use writeable::assert_writeable_eq;
    assert_writeable_eq!(LanguageIdentifier::UNKNOWN, "und");
    assert_writeable_eq!("und-001".parse::<LanguageIdentifier>().unwrap(), "und-001");
    assert_writeable_eq!(
        "und-Mymr".parse::<LanguageIdentifier>().unwrap(),
        "und-Mymr",
    );
    assert_writeable_eq!(
        "my-Mymr-MM".parse::<LanguageIdentifier>().unwrap(),
        "my-Mymr-MM",
    );
    assert_writeable_eq!(
        "my-Mymr-MM-posix".parse::<LanguageIdentifier>().unwrap(),
        "my-Mymr-MM-posix",
    );
    assert_writeable_eq!(
        "zh-macos-posix".parse::<LanguageIdentifier>().unwrap(),
        "zh-macos-posix",
    );
}
