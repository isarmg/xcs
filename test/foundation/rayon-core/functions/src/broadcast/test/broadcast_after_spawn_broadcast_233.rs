#[test]
fn broadcast_after_spawn_broadcast() {
    let (tx, rx) = channel();

    // Queue a non-blocking spawn_broadcast.
    crate::spawn_broadcast(move |ctx| tx.send(ctx.index()).unwrap());

    // This blocking broadcast runs after all prior broadcasts.
    crate::broadcast(|_| {});

    // The spawn_broadcast **must** have run by now on all threads.
    let mut v: Vec<_> = rx.try_iter().collect();
    v.sort_unstable();
    assert!(v.into_iter().eq(0..crate::current_num_threads()));
}
