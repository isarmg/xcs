#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn broadcast_sleep_race() {
    let test_duration = time::Duration::from_secs(1);
    let pool = ThreadPoolBuilder::new().num_threads(7).build().unwrap();
    let start = time::Instant::now();
    while start.elapsed() < test_duration {
        pool.broadcast(|ctx| {
            // A slight spread of sleep duration increases the chance that one
            // of the threads will race in the pool's idle sleep afterward.
            thread::sleep(time::Duration::from_micros(ctx.index() as u64));
        });
    }
}
