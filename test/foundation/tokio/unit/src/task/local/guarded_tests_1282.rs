#[cfg(all(test))]
mod tests {
    use super::*;

    // Does a `LocalSet` running on a current-thread runtime...basically work?
    //
    // This duplicates a test in `tests/task_local_set.rs`, but because this is
    // a lib test, it will run under Miri, so this is necessary to catch stacked
    // borrows violations in the `LocalSet` implementation.
    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/task/local/local_current_thread_scheduler_1291.rs"));

    // Tests that when a task on a `LocalSet` is woken by an io driver on the
    // same thread, the task is woken to the localset's local queue rather than
    // its remote queue.
    //
    // This test has to be defined in the `local.rs` file as a lib test, rather
    // than in `tests/`, because it makes assertions about the local set's
    // internal state.
    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/task/local/wakes_to_local_queue_1313.rs"));
}
