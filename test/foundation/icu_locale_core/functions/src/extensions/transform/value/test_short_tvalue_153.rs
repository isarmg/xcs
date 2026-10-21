#[test]
fn test_short_tvalue() {
    let value = Value::try_from_str("foo-longstag");
    assert!(value.is_ok());
    let value = value.unwrap();
    assert_eq!(value.0.len(), 2);
    for (s, reference) in value.0.iter().zip(&[subtag!("foo"), subtag!("longstag")]) {
        assert_eq!(s, reference);
    }

    let value = Value::try_from_str("foo-ba");
    assert!(value.is_err());
}
