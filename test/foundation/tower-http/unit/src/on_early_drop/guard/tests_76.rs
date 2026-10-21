#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn fires_on_drop() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_for_guard = fired.clone();
        {
            let _guard = OnEarlyDropGuard::new(move || {
                fired_for_guard.store(true, Ordering::Relaxed);
            });
        }
        assert!(fired.load(Ordering::Relaxed));
    }

    #[test]
    fn suppresses_when_completed() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_for_guard = fired.clone();
        {
            let mut guard = OnEarlyDropGuard::new(move || {
                fired_for_guard.store(true, Ordering::Relaxed);
            });
            guard.completed();
        }
        assert!(!fired.load(Ordering::Relaxed));
    }

    #[test]
    fn accepts_custom_on_drop_callback_impl() {
        // Verify a named struct implementing OnDropCallback works through
        // the guard, not just closures via the blanket impl.
        struct Counter(Arc<AtomicBool>);
        impl super::super::traits::OnDropCallback for Counter {
            fn on_drop(self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }

        let fired = Arc::new(AtomicBool::new(false));
        {
            let _guard = OnEarlyDropGuard::new(Counter(fired.clone()));
        }
        assert!(fired.load(Ordering::Relaxed));
    }

    // The guard must be Send and Sync whenever its callback is.
    #[allow(dead_code)]
    fn static_property_guard_is_send_sync() {
        fn assert_send<T: Send>(_: &T) {}
        fn assert_sync<T: Sync>(_: &T) {}

        let guard = OnEarlyDropGuard::new(|| {});
        assert_send(&guard);
        assert_sync(&guard);
    }
}
