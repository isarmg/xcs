#[test]
fn take_passes_errors_through() {
    let mut s = block_on_stream(iter(vec![Err(1), Err(2), Ok(3), Ok(4), Err(4)]).take(1));
    assert_eq!(s.next(), Some(Err(1)));
    assert_eq!(s.next(), Some(Err(2)));
    assert_eq!(s.next(), Some(Ok(3)));
    assert_eq!(s.next(), None);

    let mut s = block_on_stream(iter(vec![Ok(1), Err(2)]).take(1));
    assert_eq!(s.next(), Some(Ok(1)));
    assert_eq!(s.next(), None);
}
