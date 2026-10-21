#[test]
fn join_incomplete() {
    let (a, b) = oneshot::channel::<i32>();
    let (tx, rx) = channel();
    noop_waker_lw(|lw| {
        let mut f = ok(1).join(b).map(move |r| tx.send(r).unwrap());
        assert!(f.poll(lw).ok().unwrap().is_pending());
        assert!(rx.try_recv().is_err());
        a.send(2).unwrap();
        assert!(f.poll(lw).ok().unwrap().is_ready());
        assert_eq!(rx.recv().unwrap(), (1, 2));

        let (a, b) = oneshot::channel::<i32>();
        let (tx, rx) = channel();
        let mut f = b.join(Ok(2)).map(move |r| tx.send(r).unwrap());
        assert!(f.poll(lw).ok().unwrap().is_pending());
        assert!(rx.try_recv().is_err());
        a.send(1).unwrap();
        assert!(f.poll(lw).ok().unwrap().is_ready());
        assert_eq!(rx.recv().unwrap(), (1, 2));

        let (a, b) = oneshot::channel::<i32>();
        let (tx, rx) = channel();
        let mut f = ok(1).join(b).map_err(move |_r| tx.send(2).unwrap());
        assert!(f.poll(lw).ok().unwrap().is_pending());
        assert!(rx.try_recv().is_err());
        drop(a);
        assert!(f.poll(lw).is_err());
        assert_eq!(rx.recv().unwrap(), 2);

        let (a, b) = oneshot::channel::<i32>();
        let (tx, rx) = channel();
        let mut f = b.join(Ok(2)).map_err(move |_r| tx.send(1).unwrap());
        assert!(f.poll(lw).ok().unwrap().is_pending());
        assert!(rx.try_recv().is_err());
        drop(a);
        assert!(f.poll(lw).is_err());
        assert_eq!(rx.recv().unwrap(), 1);
    })
}
