use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn mutable_children_accept_a_closure_with_mutable_captured_state() {
    let seen = Arc::new(AtomicUsize::new(0));
    let observed = seen.clone();
    let mut count = 0;
    let mut children: ChildrenFnMut = ToChildren::to_children(move || {
        count += 1;
        observed.store(count, Ordering::SeqCst);
    });
    children();
    children();
    assert_eq!(seen.load(Ordering::SeqCst), 2);
}
