#[test]
fn test_debug() {
    let cases = &[
        ("hello", "\"hello\""),
        ("hello \"world\"", "\"hello \\\"world\\\"\""),
        ("\u{7FFF}hello", "\"\\xe7\\xbf\\xbfhello\""),
    ];

    for &(value, expected) in cases {
        let val = HeaderValue::from_bytes(value.as_bytes()).unwrap();
        let actual = format!("{:?}", val);
        assert_eq!(expected, actual);
    }

    let mut sensitive = HeaderValue::from_static("password");
    sensitive.set_sensitive(true);
    assert_eq!("Sensitive", format!("{:?}", sensitive));
}
