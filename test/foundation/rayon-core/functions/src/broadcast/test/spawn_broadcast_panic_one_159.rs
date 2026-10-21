#[test]
#[cfg_attr(not(panic = "unwind"), ignore)]
fn spawn_broadcast_panic_one() {
    let (tx, rx) = channel();
    let (panic_tx, panic_rx) = channel();
    let pool = ThreadPoolBuilder::new()
        .num_threads(7)
        .panic_handler(move |e| panic_tx.send(e).unwrap())
        .build()
        .unwrap();
    pool.spawn_broadcast(move |ctx| {
        tx.send(()).unwrap();
        if ctx.index() == 3 {
            panic!("Hello, world!");
        }
    });
    drop(pool); // including panic_tx
    assert_eq!(rx.into_iter().count(), 7);
    assert_eq!(panic_rx.into_iter().count(), 1);
}
