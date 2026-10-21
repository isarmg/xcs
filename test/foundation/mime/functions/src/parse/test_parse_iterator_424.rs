#[test]
fn test_parse_iterator() {
    let mut iter = MimeIter::new("application/json, application/json");
    assert_eq!(iter.next().unwrap().unwrap(), parse("application/json").unwrap());
    assert_eq!(iter.next().unwrap().unwrap(), parse("application/json").unwrap());
    assert_eq!(iter.next(), None);

    let mut iter = MimeIter::new("application/json");
    assert_eq!(iter.next().unwrap().unwrap(), parse("application/json").unwrap());
    assert_eq!(iter.next(), None);

    let mut iter = MimeIter::new("application/json;  ");
    assert_eq!(iter.next().unwrap().unwrap(), parse("application/json").unwrap());
    assert_eq!(iter.next(), None);
}
