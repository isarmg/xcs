#[test]
fn fuse() {
    let mut stream = block_on_stream(list().fuse());
    assert_eq!(stream.next(), Some(Ok(1)));
    assert_eq!(stream.next(), Some(Ok(2)));
    assert_eq!(stream.next(), Some(Ok(3)));
    assert_eq!(stream.next(), None);
    assert_eq!(stream.next(), None);
    assert_eq!(stream.next(), None);
}
