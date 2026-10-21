#[allow(deprecated)]
#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn configuration() {
    let start_handler = move |_| {};
    let exit_handler = move |_| {};
    let panic_handler = move |_| {};
    let thread_name = move |i| format!("thread_name_{i}");

    // Ensure we can call all public methods on Configuration
    crate::Configuration::new()
        .thread_name(thread_name)
        .num_threads(5)
        .panic_handler(panic_handler)
        .stack_size(4e6 as usize)
        .breadth_first()
        .start_handler(start_handler)
        .exit_handler(exit_handler)
        .build()
        .unwrap();
}
