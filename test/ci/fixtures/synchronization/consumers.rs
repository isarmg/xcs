#[test]
fn actual_consumers_share_synchronization() {
    let _subscriber = tracing_subscriber::registry();
    let once = once_cell::sync::OnceCell::new();
    assert_eq!(*once.get_or_init(|| 7), 7);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mutex = futures_intrusive::sync::Mutex::new(1, true);
        assert_eq!(*mutex.lock().await, 1);
        let mutex = tokio::sync::Mutex::new(2);
        assert_eq!(*mutex.lock().await, 2);
    });
}
