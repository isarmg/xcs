use super::*;
use std::{
    cell::Cell,
    pin::pin,
    task::{Context, Poll, Waker},
};

fn poll<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn close_before_first_poll_never_runs_task() {
    let scope = TaskScope::default();
    let ran = Cell::new(false);
    let task = scope.wrap(async { ran.set(true) }).unwrap();
    scope.close();
    assert!(poll(pin!(task)).is_ready());
    assert!(!ran.get());
    assert!(scope.wrap(async {}).is_none());
}

#[test]
fn pending_task_cannot_touch_disposed_signals() {
    use crate::prelude::*;
    let owner = Owner::new();
    let state = owner.with(|| RwSignal::new(1));
    let scope = TaskScope::default();
    let cancelled = scope.clone();
    owner.with(|| {
        let cancelled = send_wrapper::SendWrapper::new(cancelled);
        on_cleanup(move || cancelled.take().close());
    });
    let (sender, receiver) = futures::channel::oneshot::channel::<()>();
    let ran = Cell::new(false);
    let mut task = pin!(
        scope
            .wrap(async {
                let _ = receiver.await;
                state.get();
                ran.set(true);
            })
            .unwrap()
    );
    assert!(poll(task.as_mut()).is_pending());
    owner.cleanup();
    let _ = sender.send(());
    assert!(poll(task.as_mut()).is_ready());
    assert!(!ran.get());
    assert!(scope.0.borrow().pending.is_empty());
}

#[test]
fn completed_and_dropped_tasks_unregister() {
    let scope = TaskScope::default();
    let task = scope.wrap(async {}).unwrap();
    assert_eq!(scope.0.borrow().pending.len(), 1);
    assert!(poll(pin!(task)).is_ready());
    assert!(scope.0.borrow().pending.is_empty());
    let task = scope.wrap(std::future::pending()).unwrap();
    drop(task);
    assert!(scope.0.borrow().pending.is_empty());
}

#[test]
fn cancellation_drops_pending_captures_once() {
    struct Capture<'a>(&'a Cell<u32>);
    impl Drop for Capture<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Cell::new(0);
    let scope = TaskScope::default();
    let capture = Capture(&drops);
    let mut task = pin!(
        scope
            .wrap(async move {
                let _capture = capture;
                std::future::pending::<()>().await;
            })
            .unwrap()
    );
    assert!(poll(task.as_mut()).is_pending());
    scope.close();
    scope.close();
    assert!(poll(task.as_mut()).is_ready());
    assert_eq!(drops.get(), 1);
}

#[test]
fn document_scope_survives_leaving_owner_context_but_not_cleanup() {
    let owner = crate::prelude::Owner::new();
    owner.with(install);
    let scope = CURRENT
        .with(|current| current.borrow().upgrade())
        .map(TaskScope)
        .unwrap();
    assert!(scope.wrap(async {}).is_some());
    owner.cleanup();
    assert!(scope.wrap(async {}).is_none());
}
