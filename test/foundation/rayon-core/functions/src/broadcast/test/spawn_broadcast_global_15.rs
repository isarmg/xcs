#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn spawn_broadcast_global() {
    let (tx, rx) = channel();
    crate::spawn_broadcast(move |ctx| tx.send(ctx.index()).unwrap());

    let mut v: Vec<_> = rx.into_iter().collect();
    v.sort_unstable();
    assert!(v.into_iter().eq(0..crate::current_num_threads()));
}
