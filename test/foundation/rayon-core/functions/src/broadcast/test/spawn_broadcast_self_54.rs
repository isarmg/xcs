#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn spawn_broadcast_self() {
    let (tx, rx) = channel();
    let pool = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    pool.spawn(|| crate::spawn_broadcast(move |ctx| tx.send(ctx.index()).unwrap()));

    let mut v: Vec<_> = rx.into_iter().collect();
    v.sort_unstable();
    assert!(v.into_iter().eq(0..7));
}
