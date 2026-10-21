#[test]
fn stream_poll_fn() {
    let mut counter = 5usize;

    let read_stream = poll_fn(move |_| -> Poll<Option<usize>, std::io::Error> {
        if counter == 0 {
            return Ok(Poll::Ready(None));
        }
        counter -= 1;
        Ok(Poll::Ready(Some(counter)))
    });

    assert_eq!(block_on_stream(read_stream).count(), 5);
}
