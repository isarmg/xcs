//! Transactional ownership for a single document lifetime.
//!
//! Browser events can run synchronously during mounting and ready notification.
//! Closing must therefore invalidate a pending mount, not only remove a view.
#[derive(Debug, PartialEq, Eq)]
enum State<T> {
    Idle,
    Starting,
    Mounted(T),
    Closed,
}

pub(crate) struct Lifecycle<T>(State<T>);
impl<T> Lifecycle<T> {
    pub(crate) const fn new() -> Self {
        Self(State::Idle)
    }

    pub(crate) fn begin(&mut self) -> bool {
        if matches!(self.0, State::Idle) {
            self.0 = State::Starting;
            true
        } else {
            false
        }
    }

    pub(crate) fn is_starting(&self) -> bool {
        matches!(self.0, State::Starting)
    }

    pub(crate) fn is_mounted(&self) -> bool {
        matches!(self.0, State::Mounted(_))
    }

    pub(crate) fn install(&mut self, value: T) -> Result<(), T> {
        if self.is_starting() {
            self.0 = State::Mounted(value);
            Ok(())
        } else {
            Err(value)
        }
    }

    // Return resources to the caller: destructors may reenter the registry and
    // must never run while its RefCell is borrowed.
    pub(crate) fn close(&mut self) -> Option<T> {
        match std::mem::replace(&mut self.0, State::Closed) {
            State::Mounted(value) => Some(value),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn one_document_can_begin_and_publish_only_once() {
        let mut lifecycle = Lifecycle::new();
        assert!(lifecycle.begin());
        assert!(!lifecycle.begin());
        assert!(lifecycle.install(1).is_ok());
        assert!(lifecycle.is_mounted());
        assert_eq!(lifecycle.install(2), Err(2));
        assert_eq!(lifecycle.close(), Some(1));
        assert_eq!(lifecycle.close(), None);
        assert!(!lifecycle.begin());
    }

    #[test]
    fn cancellation_before_publication_rejects_late_resources() {
        let mut lifecycle = Lifecycle::new();
        assert!(lifecycle.begin());
        assert_eq!(lifecycle.close(), None);
        assert!(!lifecycle.is_starting());
        assert_eq!(lifecycle.install("late view"), Err("late view"));
        assert!(!lifecycle.is_mounted());
    }

    #[test]
    fn ready_callback_can_release_published_resources() {
        let mut lifecycle = Lifecycle::new();
        assert!(lifecycle.begin());
        lifecycle.install("owner").unwrap();
        // Equivalent to a synchronous pagehide during startup-ready dispatch.
        assert_eq!(lifecycle.close(), Some("owner"));
        assert!(!lifecycle.is_mounted());
        assert_eq!(lifecycle.install("revived owner"), Err("revived owner"));
    }

    #[test]
    fn resources_are_released_outside_the_registry_borrow() {
        struct Reentrant(Rc<RefCell<Lifecycle<Reentrant>>>);
        impl Drop for Reentrant {
            fn drop(&mut self) {
                assert!(self.0.borrow_mut().close().is_none());
            }
        }
        let registry = Rc::new(RefCell::new(Lifecycle::new()));
        registry.borrow_mut().begin();
        assert!(
            registry
                .borrow_mut()
                .install(Reentrant(registry.clone()))
                .is_ok()
        );
        let resource = registry.borrow_mut().close();
        drop(resource);
    }
}
