#[test]
#[cfg_attr(not(panic = "unwind"), ignore)]
fn panic_thread_name() {
    let (start_count, start_handler) = count_handler();
    let (exit_count, exit_handler) = count_handler();
    let builder = ThreadPoolBuilder::new()
        .num_threads(10)
        .start_handler(start_handler)
        .exit_handler(exit_handler)
        .thread_name(|i| {
            if i >= 5 {
                panic!();
            }
            format!("panic_thread_name#{i}")
        });

    let pool = crate::unwind::halt_unwinding(|| builder.build());
    assert!(pool.is_err(), "thread-name panic should propagate!");

    // Assuming they're created in order, threads 0 through 4 should have
    // been started already, and then terminated by the panic.
    assert_eq!(5, wait_for_counter(start_count));
    assert_eq!(5, wait_for_counter(exit_count));
}
