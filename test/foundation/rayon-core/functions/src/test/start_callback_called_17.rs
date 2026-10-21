#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn start_callback_called() {
    let n_threads = 16;
    let n_called = Arc::new(AtomicUsize::new(0));
    // Wait for all the threads in the pool plus the one running tests.
    let barrier = Arc::new(Barrier::new(n_threads + 1));

    let b = Arc::clone(&barrier);
    let nc = Arc::clone(&n_called);
    let start_handler = move |_| {
        nc.fetch_add(1, Ordering::SeqCst);
        b.wait();
    };

    let conf = ThreadPoolBuilder::new()
        .num_threads(n_threads)
        .start_handler(start_handler);
    let _ = conf.build().unwrap();

    // Wait for all the threads to have been scheduled to run.
    barrier.wait();

    // The handler must have been called on every started thread.
    assert_eq!(n_called.load(Ordering::SeqCst), n_threads);
}
