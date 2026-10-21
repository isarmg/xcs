#[test]
fn test_parse_iterator_all_invalid() {
    let mut iter = MimeIter::new("application/json, text/html");
    assert_eq!(iter.next().unwrap().unwrap_err(), "application/json");
    assert_eq!(iter.next(), None);
}
