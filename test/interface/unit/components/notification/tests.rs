use super::*;
use std::{cell::Cell, rc::Rc};

struct Timer(Rc<Cell<usize>>);
impl Timer {
    fn new(active: &Rc<Cell<usize>>) -> Self {
        active.set(active.get() + 1);
        assert!(
            active.get() <= MAX_TOASTS,
            "live timers must remain bounded"
        );
        Self(active.clone())
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}
fn queue() -> ToastQueue<Timer> {
    ToastQueue {
        sequence: 0,
        entries: Vec::new(),
    }
}

#[test]
fn burst_eviction_dismissal_expiry_and_clear_release_timers() {
    let active = Rc::new(Cell::new(0));
    let mut queue = queue();
    for _ in 0..10_000 {
        queue.push("通知🙂".repeat(300), |_| Timer::new(&active));
    }
    assert_eq!(active.get(), 5);
    assert_eq!(queue.records().len(), 5);
    assert_eq!(queue.records()[0].id, 9_996);
    assert_eq!(queue.records()[0].message.chars().count(), 512);
    queue.dismiss(9_996); // manual close
    queue.dismiss(9_997); // expiry uses the same path
    queue.dismiss(1); // stale expiry for an evicted item is harmless
    assert_eq!(active.get(), 3);
    queue.entries.clear(); // logout
    assert_eq!(active.get(), 0);
}

#[test]
fn stale_expiry_after_owner_disposal_is_harmless() {
    let owner = Owner::new();
    let toasts = owner.with(Toasts::new);
    toasts.dismiss(1);
    toasts.clear();
    drop(owner);
    toasts.dismiss(1);
    toasts.clear();
}

#[test]
fn owner_disposal_cancels_pending_notification_timers() {
    let active = Rc::new(Cell::new(0));
    let owner = Owner::new();
    owner.with(|| {
        let mut queue = queue();
        for _ in 0..MAX_TOASTS {
            queue.push("pending".into(), |_| Timer::new(&active));
        }
        let _queue = StoredValue::new(SendWrapper::new(queue));
    });
    assert_eq!(active.get(), MAX_TOASTS);
    drop(owner);
    assert_eq!(active.get(), 0);
}
