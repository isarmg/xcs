//! UI tasks owned by this document's root, not by the current render effect.
//!
//! Closing the root cancels pending UI futures before they can touch disposed
//! signals. It does not establish whether an already issued server operation
//! committed, abort a transport, or retry an operation. An already requested
//! server logout deliberately uses a detached task to finish revocation.
use futures::future::{AbortHandle, Abortable};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    future::Future,
    rc::{Rc, Weak},
};

thread_local! {
    static CURRENT: RefCell<Weak<RefCell<Inner>>> = const { RefCell::new(Weak::new()) };
}

#[derive(Default)]
struct Inner {
    closed: bool,
    next: u64,
    pending: BTreeMap<u64, AbortHandle>,
}

#[derive(Clone, Default)]
struct TaskScope(Rc<RefCell<Inner>>);

impl TaskScope {
    #[cfg(any(target_arch = "wasm32", test))]
    fn close(&self) {
        let pending = {
            let mut inner = self.0.borrow_mut();
            inner.closed = true;
            std::mem::take(&mut inner.pending)
        };
        // Waking/dropping tasks must never happen under the registry borrow.
        for (_, handle) in pending {
            handle.abort();
        }
    }

    fn wrap<F: Future<Output = ()>>(&self, future: F) -> Option<impl Future<Output = ()> + use<F>> {
        let (handle, registration) = AbortHandle::new_pair();
        let id = {
            let mut inner = self.0.borrow_mut();
            if inner.closed {
                return None;
            }
            let id = inner.next;
            inner.next = inner
                .next
                .checked_add(1)
                .expect("UI task counter exhausted");
            inner.pending.insert(id, handle);
            id
        };
        let guard = Registration {
            scope: Rc::downgrade(&self.0),
            id,
        };
        Some(async move {
            let _guard = guard;
            let _ = Abortable::new(future, registration).await;
        })
    }
}

struct Registration {
    scope: Weak<RefCell<Inner>>,
    id: u64,
}
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(scope) = self.scope.upgrade() {
            scope.borrow_mut().pending.remove(&self.id);
        }
    }
}

/// Called once inside the application root owner. The runtime's single-document
/// lifecycle prevents replacing this scope or attaching old callbacks to a new root.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn install() {
    let scope = TaskScope::default();
    CURRENT.with(|current| *current.borrow_mut() = Rc::downgrade(&scope.0));
    let scope = send_wrapper::SendWrapper::new(scope);
    crate::prelude::on_cleanup(move || scope.take().close());
}

/// Schedule a UI continuation for the mounted document, including from event
/// callbacks with no current reactive owner. After teardown (or before mounting)
/// the future is dropped without polling. This does not promise network rollback.
pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    let scope = CURRENT
        .with(|current| current.borrow().upgrade())
        .map(TaskScope);
    if let Some(task) = scope.and_then(|scope| scope.wrap(future)) {
        wasm_bindgen_futures::spawn_local(task);
    }
}

#[cfg(test)]
mod tests {
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
}
