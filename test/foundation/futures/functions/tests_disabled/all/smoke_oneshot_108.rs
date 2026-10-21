#[test]
fn smoke_oneshot() {
    assert_done(
        || {
            let (c, p) = oneshot::channel();
            c.send(1).unwrap();
            p
        },
        Ok(1),
    );
    assert_done(
        || {
            let (c, p) = oneshot::channel::<i32>();
            drop(c);
            p
        },
        Err(Canceled),
    );
    let mut completes = Vec::new();
    assert_empty(|| {
        let (a, b) = oneshot::channel::<i32>();
        completes.push(a);
        b
    });

    let (c, mut p) = oneshot::channel::<i32>();
    drop(c);
    let res = panic_waker_lw(|lw| p.poll(lw));
    assert!(res.is_err());
    let (c, p) = oneshot::channel::<i32>();
    drop(c);
    let (tx, rx) = channel();
    p.then(move |_| tx.send(())).forget();
    rx.recv().unwrap();
}
