#[test]
fn join_cancels() {
    let ((a, b), (c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
    let ((btx, _brx), (dtx, drx)) = (channel(), channel());
    let b = b.map(move |b| {
        btx.send(b).unwrap();
        b
    });
    let d = d.map(move |d| {
        dtx.send(d).unwrap();
        d
    });

    let mut f = b.join(d);
    drop(a);
    let res = panic_waker_lw(|lw| f.poll(lw));
    assert!(res.is_err());
    drop(c);
    assert!(drx.recv().is_err());

    let ((a, b), (c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
    let ((btx, _brx), (dtx, drx)) = (channel(), channel());
    let b = b.map(move |b| {
        btx.send(b).unwrap();
        b
    });
    let d = d.map(move |d| {
        dtx.send(d).unwrap();
        d
    });

    let (tx, rx) = channel();
    let f = b.join(d);
    f.then(move |_| {
        tx.send(()).unwrap();
        let res: Result<(), ()> = Ok(());
        res
    })
    .forget();
    assert!(rx.try_recv().is_err());
    drop(a);
    rx.recv().unwrap();
    drop(c);
    assert!(drx.recv().is_err());
}
