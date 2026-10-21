#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn broadcast_mutual_sleepy() {
    let count = AtomicUsize::new(0);
    let pool1 = ThreadPoolBuilder::new().num_threads(3).build().unwrap();
    let pool2 = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    pool1.install(|| {
        thread::sleep(time::Duration::from_secs(1));
        pool2.broadcast(|_| {
            thread::sleep(time::Duration::from_secs(1));
            pool1.broadcast(|_| {
                thread::sleep(time::Duration::from_millis(100));
                count.fetch_add(1, Ordering::Relaxed);
            })
        })
    });
    assert_eq!(count.into_inner(), 3 * 7);
}
