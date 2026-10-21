#[test]
fn test_writeable() {
    use crate::Locale;
    use writeable::assert_writeable_eq;
    assert_writeable_eq!(Extensions::new(), "");
    assert_writeable_eq!(
        "my-t-my-d0-zawgyi".parse::<Locale>().unwrap().extensions,
        "t-my-d0-zawgyi",
    );
    assert_writeable_eq!(
        "ar-SA-u-ca-islamic-civil"
            .parse::<Locale>()
            .unwrap()
            .extensions,
        "u-ca-islamic-civil",
    );
    assert_writeable_eq!(
        "en-001-x-foo-bar".parse::<Locale>().unwrap().extensions,
        "x-foo-bar",
    );
    assert_writeable_eq!(
        "und-t-m0-true".parse::<Locale>().unwrap().extensions,
        "t-m0-true",
    );
    assert_writeable_eq!(
        "und-a-foo-t-foo-u-foo-w-foo-z-foo-x-foo"
            .parse::<Locale>()
            .unwrap()
            .extensions,
        "a-foo-t-foo-u-foo-w-foo-z-foo-x-foo",
    );
    assert_writeable_eq!(
        "en-1-ext-value".parse::<Locale>().unwrap().extensions,
        "1-ext-value",
    );
    assert_writeable_eq!(
        "und-a-foo-1-bar".parse::<Locale>().unwrap().extensions,
        "1-bar-a-foo",
    );
}
