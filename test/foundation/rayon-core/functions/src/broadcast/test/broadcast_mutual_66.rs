#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn broadcast_mutual() {
    let count = AtomicUsize::new(0);
    let pool1 = ThreadPoolBuilder::new().num_threads(3).build().unwrap();
    let pool2 = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    pool1.install(|| {
        pool2.broadcast(|_| {
            pool1.broadcast(|_| {
                count.fetch_add(1, Ordering::Relaxed);
            })
        })
    });
    assert_eq!(count.into_inner(), 3 * 7);
}
