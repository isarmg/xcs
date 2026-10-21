#[test]
fn broadcast_after_spawn() {
    let (tx, rx) = channel();

    // Queue a regular spawn on a thread-local deque.
    crate::registry::in_worker(move |_, _| {
        crate::spawn(move || tx.send(22).unwrap());
    });

    // Broadcast runs after the local deque is empty.
    crate::broadcast(|_| {});

    // The spawn **must** have run by now.
    assert_eq!(22, rx.try_recv().unwrap());
}
