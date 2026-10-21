#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn worker_thread_index() {
    let pool = ThreadPoolBuilder::new().num_threads(22).build().unwrap();
    assert_eq!(pool.current_num_threads(), 22);
    assert_eq!(pool.current_thread_index(), None);
    let index = pool.install(|| pool.current_thread_index().unwrap());
    assert!(index < 22);
}
