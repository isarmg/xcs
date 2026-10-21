#[test]
fn skip_passes_errors_through() {
    let mut s = block_on_stream(iter(vec![Err(1), Err(2), Ok(3), Ok(4), Ok(5)]).skip(1));
    assert_eq!(s.next(), Some(Err(1)));
    assert_eq!(s.next(), Some(Err(2)));
    assert_eq!(s.next(), Some(Ok(4)));
    assert_eq!(s.next(), Some(Ok(5)));
    assert_eq!(s.next(), None);
}
