#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn broadcast_pool() {
    let pool = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    let v = pool.broadcast(|ctx| ctx.index());
    assert!(v.into_iter().eq(0..7));
}
