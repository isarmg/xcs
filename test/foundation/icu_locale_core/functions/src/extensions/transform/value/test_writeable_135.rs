#[test]
fn test_writeable() {
    use writeable::assert_writeable_eq;

    let hybrid = "hybrid".parse().unwrap();
    let foobar = "foobar".parse().unwrap();

    assert_writeable_eq!(Value::default(), "true");
    assert_writeable_eq!(
        Value::from_short_slice_unchecked(vec![hybrid].into()),
        "hybrid"
    );
    assert_writeable_eq!(
        Value::from_short_slice_unchecked(vec![hybrid, foobar].into()),
        "hybrid-foobar"
    );
}
