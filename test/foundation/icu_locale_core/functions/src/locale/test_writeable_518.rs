#[test]
fn test_writeable() {
    use writeable::assert_writeable_eq;
    assert_writeable_eq!(Locale::UNKNOWN, "und");
    assert_writeable_eq!("und-001".parse::<Locale>().unwrap(), "und-001");
    assert_writeable_eq!("und-Mymr".parse::<Locale>().unwrap(), "und-Mymr");
    assert_writeable_eq!("my-Mymr-MM".parse::<Locale>().unwrap(), "my-Mymr-MM");
    assert_writeable_eq!(
        "my-Mymr-MM-posix".parse::<Locale>().unwrap(),
        "my-Mymr-MM-posix",
    );
    assert_writeable_eq!(
        "zh-macos-posix".parse::<Locale>().unwrap(),
        "zh-macos-posix",
    );
    assert_writeable_eq!(
        "my-t-my-d0-zawgyi".parse::<Locale>().unwrap(),
        "my-t-my-d0-zawgyi",
    );
    assert_writeable_eq!(
        "ar-SA-u-ca-islamic-civil".parse::<Locale>().unwrap(),
        "ar-SA-u-ca-islamic-civil",
    );
    assert_writeable_eq!(
        "en-001-x-foo-bar".parse::<Locale>().unwrap(),
        "en-001-x-foo-bar",
    );
    assert_writeable_eq!("und-t-m0-true".parse::<Locale>().unwrap(), "und-t-m0-true",);
}
