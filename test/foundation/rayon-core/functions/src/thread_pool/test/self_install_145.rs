#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn self_install() {
    let pool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();

    // If the inner `install` blocks, then nothing will actually run it!
    assert!(pool.install(|| pool.install(|| true)));
}
