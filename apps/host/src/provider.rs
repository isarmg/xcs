//! Balance a provider access attempt even when opening or copying fails.
struct Cleanup<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for Cleanup<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.0.take() {
            cleanup();
        }
    }
}
pub(super) fn with_cleanup<T>(action: impl FnOnce() -> T, cleanup: impl FnOnce()) -> T {
    let _cleanup = Cleanup(Some(cleanup));
    action()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn cleanup_covers_open_failure_copy_failure_and_success() {
        for outcome in [Err("open failed"), Err("copy failed"), Ok(())] {
            let stopped = Cell::new(0);
            let result = with_cleanup(|| outcome, || stopped.set(stopped.get() + 1));
            assert_eq!(result, outcome);
            assert_eq!(stopped.get(), 1);
        }
    }
    #[test]
    fn cleanup_runs_during_unwinding() {
        let stopped = Cell::new(false);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_cleanup(|| panic!("provider panicked"), || stopped.set(true));
        }));
        assert!(result.is_err());
        assert!(stopped.get());
    }
}
