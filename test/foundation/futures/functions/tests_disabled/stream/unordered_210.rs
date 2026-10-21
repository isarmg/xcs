#[test]
fn unordered() {
    let (tx, rx) = mpsc::channel(1);
    let (a, b) = oneshot::channel::<u32>();
    let (c, d) = oneshot::channel::<u32>();

    tx.send(Box::new(b.recover(|_| panic!())) as Box<Future<Item = _, Error = _> + Send>)
        .and_then(|tx| tx.send(Box::new(d.recover(|_| panic!()))))
        .forget();

    let mut rx = rx.buffer_unordered(2);
    sassert_empty(&mut rx);
    let mut rx = block_on_stream(rx);
    c.send(3).unwrap();
    assert_eq!(rx.next(), Some(Ok(3)));
    a.send(5).unwrap();
    assert_eq!(rx.next(), Some(Ok(5)));
    assert_eq!(rx.next(), None);

    let (tx, rx) = mpsc::channel(1);
    let (a, b) = oneshot::channel::<u32>();
    let (c, d) = oneshot::channel::<u32>();

    tx.send(Box::new(b.recover(|_| panic!())) as Box<Future<Item = _, Error = _> + Send>)
        .and_then(|tx| tx.send(Box::new(d.recover(|_| panic!()))))
        .forget();

    // We don't even get to see `c` until `a` completes.
    let mut rx = rx.buffer_unordered(1);
    sassert_empty(&mut rx);
    c.send(3).unwrap();
    sassert_empty(&mut rx);
    a.send(5).unwrap();
    let mut rx = block_on_stream(rx);
    assert_eq!(rx.next(), Some(Ok(5)));
    assert_eq!(rx.next(), Some(Ok(3)));
    assert_eq!(rx.next(), None);
}
