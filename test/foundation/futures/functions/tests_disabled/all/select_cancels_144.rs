#[test]
fn select_cancels() {
    let ((a, b), (c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
    let ((btx, brx), (dtx, drx)) = (channel(), channel());
    let b = b.map(move |b| {
        btx.send(b).unwrap();
        b
    });
    let d = d.map(move |d| {
        dtx.send(d).unwrap();
        d
    });

    let mut f = b.select(d).then(unselect);
    // assert!(f.poll(&mut Task::new()).is_pending());
    assert!(brx.try_recv().is_err());
    assert!(drx.try_recv().is_err());
    a.send(1).unwrap();
    noop_waker_lw(|lw| {
        let res = f.poll(lw);
        assert!(res.ok().unwrap().is_ready());
        assert_eq!(brx.recv().unwrap(), 1);
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

        let mut f = b.select(d).then(unselect);
        assert!(f.poll(lw).ok().unwrap().is_pending());
        assert!(f.poll(lw).ok().unwrap().is_pending());
        a.send(1).unwrap();
        assert!(f.poll(lw).ok().unwrap().is_ready());
        drop((c, f));
        assert!(drx.recv().is_err());
    })
}
