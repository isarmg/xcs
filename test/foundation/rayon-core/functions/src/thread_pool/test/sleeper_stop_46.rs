#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn sleeper_stop() {
    use std::{thread, time};

    let registry;

    {
        // once we exit this block, thread pool will be dropped
        let thread_pool = ThreadPoolBuilder::new().num_threads(22).build().unwrap();
        registry = Arc::clone(&thread_pool.registry);

        // Give time for at least some of the thread pool to fall asleep.
        thread::sleep(time::Duration::from_secs(1));
    }

    // once thread pool is dropped, registry should terminate, which
    // should lead to worker threads stopping
    registry.wait_until_stopped();
}
