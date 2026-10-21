#[test]
fn test_parse_iterator_invalid() {
    let mut iter = MimeIter::new("application/json, invalid, application/json");
    assert_eq!(iter.next().unwrap().unwrap(), parse("application/json").unwrap());
    assert_eq!(iter.next().unwrap().unwrap_err(), "invalid");
    assert_eq!(iter.next().unwrap().unwrap(), parse("application/json").unwrap());
    assert_eq!(iter.next(), None);
}
