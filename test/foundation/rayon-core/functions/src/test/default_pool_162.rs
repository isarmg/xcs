#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn default_pool() {
    ThreadPoolBuilder::default().build().unwrap();
}
