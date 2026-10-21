#[test]
fn yield_now_to_spawn() {
    let (tx, rx) = channel();

    // Queue a regular spawn.
    crate::spawn(move || tx.send(22).unwrap());

    // The single-threaded fallback mode (for wasm etc.) won't
    // get a chance to run the spawn if we never yield to it.
    crate::registry::in_worker(move |_, _| {
        crate::yield_now();
    });

    // The spawn **must** have started by now, but we still might have to wait
    // for it to finish if a different thread stole it first.
    assert_eq!(22, rx.recv().unwrap());
}
