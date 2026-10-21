#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn workers_stop() {
    let registry;

    {
        // once we exit this block, thread pool will be dropped
        let thread_pool = ThreadPoolBuilder::new().num_threads(22).build().unwrap();
        registry = thread_pool.install(|| {
            // do some work on these threads
            join_a_lot(22);

            Arc::clone(&thread_pool.registry)
        });
        assert_eq!(registry.num_threads(), 22);
    }

    // once thread pool is dropped, registry should terminate, which
    // should lead to worker threads stopping
    registry.wait_until_stopped();
}
