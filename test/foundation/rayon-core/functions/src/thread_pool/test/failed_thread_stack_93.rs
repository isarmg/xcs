#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn failed_thread_stack() {
    // Note: we first tried to force failure with a `usize::MAX` stack, but
    // macOS and Windows weren't fazed, or at least didn't fail the way we want.
    // They work with `isize::MAX`, but 32-bit platforms may feasibly allocate a
    // 2GB stack, so it might not fail until the second thread.
    let stack_size = isize::MAX as usize;

    let (start_count, start_handler) = count_handler();
    let (exit_count, exit_handler) = count_handler();
    let builder = ThreadPoolBuilder::new()
        .num_threads(10)
        .stack_size(stack_size)
        .start_handler(start_handler)
        .exit_handler(exit_handler);

    let pool = builder.build();
    assert!(pool.is_err(), "thread stack should have failed!");

    // With such a huge stack, 64-bit will probably fail on the first thread;
    // 32-bit might manage the first 2GB, but certainly fail the second.
    let start_count = wait_for_counter(start_count);
    assert!(start_count <= 1);
    assert_eq!(start_count, wait_for_counter(exit_count));
}
