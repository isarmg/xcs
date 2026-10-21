#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn spawn_broadcast_mutual_sleepy() {
    let (tx, rx) = channel();
    let pool1 = Arc::new(ThreadPoolBuilder::new().num_threads(3).build().unwrap());
    let pool2 = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    pool1.spawn({
        let pool1 = Arc::clone(&pool1);
        move || {
            thread::sleep(time::Duration::from_secs(1));
            pool2.spawn_broadcast(move |_| {
                let tx = tx.clone();
                thread::sleep(time::Duration::from_secs(1));
                pool1.spawn_broadcast(move |_| {
                    thread::sleep(time::Duration::from_millis(100));
                    tx.send(()).unwrap();
                })
            })
        }
    });
    assert_eq!(rx.into_iter().count(), 3 * 7);
}
