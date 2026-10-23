use task_dispatch::Executor;

#[test]
fn uninitialized_dispatch_fails_loudly_on_every_thread() {
    // Each integration test binary owns its process-wide initialization state.
    for _ in 0..4 {
        std::thread::spawn(|| {
            assert!(std::panic::catch_unwind(|| Executor::spawn(async {})).is_err());
            assert!(std::panic::catch_unwind(|| Executor::spawn_local(async {})).is_err());
            Executor::poll_local();
        })
        .join()
        .unwrap();
    }
}
