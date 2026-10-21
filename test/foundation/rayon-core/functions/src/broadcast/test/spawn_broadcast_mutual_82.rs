#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn spawn_broadcast_mutual() {
    let (tx, rx) = channel();
    let pool1 = Arc::new(ThreadPoolBuilder::new().num_threads(3).build().unwrap());
    let pool2 = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    pool1.spawn({
        let pool1 = Arc::clone(&pool1);
        move || {
            pool2.spawn_broadcast(move |_| {
                let tx = tx.clone();
                pool1.spawn_broadcast(move |_| tx.send(()).unwrap())
            })
        }
    });
    assert_eq!(rx.into_iter().count(), 3 * 7);
}
