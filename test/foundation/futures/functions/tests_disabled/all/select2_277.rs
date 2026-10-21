#[test]
fn select2() {
    assert_done(|| f_ok(2).select(empty()).then(unselect), Ok(2));
    assert_done(|| empty().select(f_ok(2)).then(unselect), Ok(2));
    assert_done(|| f_err(2).select(empty()).then(unselect), Err(2));
    assert_done(|| empty().select(f_err(2)).then(unselect), Err(2));

    assert_done(
        || {
            f_ok(1).select(f_ok(2)).map_err(|_| 0).and_then(|either_tup| {
                let (a, b) = either_tup.into_inner();
                b.map(move |b| a + b)
            })
        },
        Ok(3),
    );

    // Finish one half of a select and then fail the second, ensuring that we
    // get the notification of the second one.
    {
        let ((a, b), (c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
        let f = b.select(d);
        let (tx, rx) = channel();
        f.map(move |r| tx.send(r).unwrap()).forget();
        a.send(1).unwrap();
        let (val, next) = rx.recv().unwrap().into_inner();
        assert_eq!(val, 1);
        let (tx, rx) = channel();
        next.map_err(move |_r| tx.send(2).unwrap()).forget();
        assert_eq!(rx.try_recv().err().unwrap(), TryRecvError::Empty);
        drop(c);
        assert_eq!(rx.recv().unwrap(), 2);
    }

    // Fail the second half and ensure that we see the first one finish
    {
        let ((a, b), (c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
        let f = b.select(d);
        let (tx, rx) = channel();
        f.map_err(move |r| tx.send((1, r.into_inner().1)).unwrap()).forget();
        drop(c);
        let (val, next) = rx.recv().unwrap();
        assert_eq!(val, 1);
        let (tx, rx) = channel();
        next.map(move |r| tx.send(r).unwrap()).forget();
        assert_eq!(rx.try_recv().err().unwrap(), TryRecvError::Empty);
        a.send(2).unwrap();
        assert_eq!(rx.recv().unwrap(), 2);
    }

    // Cancelling the first half should cancel the second
    {
        let ((_a, b), (_c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
        let ((btx, brx), (dtx, drx)) = (channel(), channel());
        let b = b.map(move |v| {
            btx.send(v).unwrap();
            v
        });
        let d = d.map(move |v| {
            dtx.send(v).unwrap();
            v
        });
        let f = b.select(d);
        drop(f);
        assert!(drx.recv().is_err());
        assert!(brx.recv().is_err());
    }

    // Cancel after a schedule
    {
        let ((_a, b), (_c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
        let ((btx, brx), (dtx, drx)) = (channel(), channel());
        let b = b.map(move |v| {
            btx.send(v).unwrap();
            v
        });
        let d = d.map(move |v| {
            dtx.send(v).unwrap();
            v
        });
        let mut f = b.select(d);
        let _res = noop_waker_lw(|lw| f.poll(lw));
        drop(f);
        assert!(drx.recv().is_err());
        assert!(brx.recv().is_err());
    }

    // Cancel propagates
    {
        let ((a, b), (_c, d)) = (oneshot::channel::<i32>(), oneshot::channel::<i32>());
        let ((btx, brx), (dtx, drx)) = (channel(), channel());
        let b = b.map(move |v| {
            btx.send(v).unwrap();
            v
        });
        let d = d.map(move |v| {
            dtx.send(v).unwrap();
            v
        });
        let (tx, rx) = channel();
        b.select(d).map(move |_| tx.send(()).unwrap()).forget();
        drop(a);
        assert!(drx.recv().is_err());
        assert!(brx.recv().is_err());
        assert!(rx.recv().is_err());
    }

    // Cancel on early drop
    {
        let (tx, rx) = channel();
        let f = f_ok(1).select(empty::<_, ()>().map(move |()| {
            tx.send(()).unwrap();
            1
        }));
        drop(f);
        assert!(rx.recv().is_err());
    }
}
