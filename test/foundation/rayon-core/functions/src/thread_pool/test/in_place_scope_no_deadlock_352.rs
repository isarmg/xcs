#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn in_place_scope_no_deadlock() {
    let pool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let (tx, rx) = channel();
    let rx_ref = &rx;
    pool.in_place_scope(move |s| {
        // With regular scopes this closure would never run because this scope op
        // itself would block the only worker thread.
        s.spawn(move |_| {
            tx.send(()).unwrap();
        });
        rx_ref.recv().unwrap();
    });
}
