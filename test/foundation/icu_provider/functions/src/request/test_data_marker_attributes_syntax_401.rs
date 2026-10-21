#[test]
fn test_data_marker_attributes_syntax() {
    let valid_cases = [
        "long-meter",
        "long",
        "meter",
        "short-meter-second",
        "usd",
        "nested/part",
        "foo/bar/baz",
        "",
    ];

    let invalid_cases = [
        "/leading",
        "trailing/",
        "double//slash",
        "invalid space",
        "invalid$character",
        "invalid\\backslash",
    ];

    for s in valid_cases {
        assert!(
            DataMarkerAttributes::try_from_str(s).is_ok(),
            "Expected valid: {}",
            s
        );
    }

    for s in invalid_cases {
        assert!(
            DataMarkerAttributes::try_from_str(s).is_err(),
            "Expected invalid: {}",
            s
        );
    }
}
