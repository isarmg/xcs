//! Intrusive linked list-based implementation of `event-listener`.
//!
//! This implementation crates an intrusive linked list of listeners. This list
//! is secured using either a libstd mutex or a critical section.

use crate::event::notify::{GenericNotify, Internal, Notification};
use crate::event::sync::atomic::Ordering;
use crate::event::sync::cell::{Cell, UnsafeCell};
use crate::event::{RegisterResult, State, TaskRef};

#[cfg(all(feature = "std"))]
use core::ops::{Deref, DerefMut};

use core::marker::PhantomPinned;
use core::mem;
use core::pin::Pin;
use core::ptr::NonNull;

pub(super) struct List<T>(
    /// libstd-based implementation uses a normal Mutex to secure the data.
    #[cfg(all(feature = "std"))]
    crate::event::sync::Mutex<Inner<T>>,
    /// Spinlock implementation uses a spinlock.
    #[cfg(not(any(feature = "std")))]
    spinlock::Spinlock<Inner<T>>,
);

struct Inner<T> {
    /// The head of the linked list.
    head: Option<NonNull<Link<T>>>,

    /// The tail of the linked list.
    tail: Option<NonNull<Link<T>>>,

    /// The first unnotified listener.
    next: Option<NonNull<Link<T>>>,

    /// Total number of listeners.
    len: usize,

    /// The number of notified listeners.
    notified: usize,
}

impl<T> List<T> {
    /// Create a new, empty event listener list.
    pub(super) fn new() -> Self {
        let inner = Inner {
            head: None,
            tail: None,
            next: None,
            len: 0,
            notified: 0,
        };

        #[cfg(all(feature = "std"))]
        {
            Self(crate::event::sync::Mutex::new(inner))
        }

        #[cfg(not(any(feature = "std")))]
        {
            Self(spinlock::Spinlock::new(inner))
        }
    }

    /// Get the total number of listeners without blocking.
    #[cfg(all(feature = "std"))]
    pub(crate) fn try_total_listeners(&self) -> Option<usize> {
        self.0.try_lock().ok().map(|list| list.len)
    }

    /// Get the total number of listeners without blocking.
    #[cfg(not(any(feature = "std")))]
    pub(crate) fn try_total_listeners(&self) -> Option<usize> {
        self.0.try_with(|list| list.len)
    }

    /// Get the total number of listeners with blocking.
    #[cfg(all(feature = "std"))]
    pub(crate) fn total_listeners(&self) -> usize {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).len
    }
}

impl<T> crate::event::Inner<T> {
    #[cfg(all(feature = "std"))]
    fn with_inner<R>(&self, f: impl FnOnce(&mut Inner<T>) -> R) -> R {
        struct ListLock<'a, 'b, T> {
            lock: crate::event::sync::MutexGuard<'a, Inner<T>>,
            inner: &'b crate::event::Inner<T>,
        }

        impl<T> Deref for ListLock<'_, '_, T> {
            type Target = Inner<T>;

            fn deref(&self) -> &Self::Target {
                &self.lock
            }
        }

        impl<T> DerefMut for ListLock<'_, '_, T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.lock
            }
        }

        impl<T> Drop for ListLock<'_, '_, T> {
            fn drop(&mut self) {
                update_notified(&self.inner.notified, &self.lock);
            }
        }

        let mut list = ListLock {
            inner: self,
            lock: self.list.0.lock().unwrap_or_else(|e| e.into_inner()),
        };
        f(&mut list)
    }

    #[cfg(any(not(feature = "std")))]
    fn with_inner<R>(&self, f: impl FnOnce(&mut Inner<T>) -> R) -> R {
        struct ListWrapper<'a, T> {
            inner: &'a crate::event::Inner<T>,
            list: &'a mut Inner<T>,
        }

        impl<T> Drop for ListWrapper<'_, T> {
            fn drop(&mut self) {
                update_notified(&self.inner.notified, self.list);
            }
        }

        self.list.0.with(move |list| {
            let wrapper = ListWrapper {
                inner: self,
                list: &mut *list,
            };

            f(wrapper.list)
        })
    }

    /// Add a new listener to the list.
    pub(crate) fn insert(&self, mut listener: Pin<&mut Option<Listener<T>>>) {
        self.with_inner(|inner| {
            listener.as_mut().set(Some(Listener {
                link: UnsafeCell::new(Link {
                    state: Cell::new(State::Created),
                    prev: Cell::new(inner.tail),
                    next: Cell::new(None),
                }),
                _pin: PhantomPinned,
            }));
            let listener = listener.as_pin_mut().unwrap();

            {
                let entry_guard = listener.link.get();
                // SAFETY: We are locked, so we can access the inner `link`.
                let entry = unsafe { entry_guard.deref() };

                // Replace the tail with the new entry.
                match inner.tail.replace(entry.into()) {
                    None => inner.head = Some(entry.into()),
                    Some(t) => unsafe { t.as_ref().next.set(Some(entry.into())) },
                };
            }

            // If there are no unnotified entries, this is the first one.
            if inner.next.is_none() {
                inner.next = inner.tail;
            }

            // Bump the entry count.
            inner.len += 1;
        });
    }

    /// Remove a listener from the list.
    pub(crate) fn remove(
        &self,
        listener: Pin<&mut Option<Listener<T>>>,
        propagate: bool,
    ) -> Option<State<T>> {
        self.with_inner(|inner| inner.remove(listener, propagate))
    }

    /// Notifies a number of entries.
    #[cold]
    pub(crate) fn notify(&self, notify: impl Notification<Tag = T>) -> usize {
        self.with_inner(|inner| inner.notify(notify))
    }

    /// Register a task to be notified when the event is triggered.
    ///
    /// Returns `true` if the listener was already notified, and `false` otherwise. If the listener
    /// isn't inserted, returns `None`.
    pub(crate) fn register(
        &self,
        mut listener: Pin<&mut Option<Listener<T>>>,
        task: TaskRef<'_>,
    ) -> RegisterResult<T> {
        self.with_inner(|inner| {
            let entry_guard = match listener.as_mut().as_pin_mut() {
                Some(listener) => listener.link.get(),
                None => return RegisterResult::NeverInserted,
            };
            // SAFETY: We are locked, so we can access the inner `link`.
            let entry = unsafe { entry_guard.deref() };

            // Take out the state and check it.
            match entry.state.replace(State::NotifiedTaken) {
                State::Notified { tag, .. } => {
                    // We have been notified, remove the listener.
                    inner.remove(listener, false);
                    RegisterResult::Notified(tag)
                }

                State::Task(other_task) => {
                    // Only replace the task if it's different.
                    entry.state.set(State::Task({
                        if !task.will_wake(other_task.as_task_ref()) {
                            task.into_task()
                        } else {
                            other_task
                        }
                    }));

                    RegisterResult::Registered
                }

                _ => {
                    // We have not been notified, register the task.
                    entry.state.set(State::Task(task.into_task()));
                    RegisterResult::Registered
                }
            }
        })
    }
}

impl<T> Inner<T> {
    fn remove(
        &mut self,
        mut listener: Pin<&mut Option<Listener<T>>>,
        propagate: bool,
    ) -> Option<State<T>> {
        let entry_guard = listener.as_mut().as_pin_mut()?.link.get();
        let entry = unsafe { entry_guard.deref() };

        let prev = entry.prev.get();
        let next = entry.next.get();

        // Unlink from the previous entry.
        match prev {
            None => self.head = next,
            Some(p) => unsafe {
                p.as_ref().next.set(next);
            },
        }

        // Unlink from the next entry.
        match next {
            None => self.tail = prev,
            Some(n) => unsafe {
                n.as_ref().prev.set(prev);
            },
        }

        // If this was the first unnotified entry, update the next pointer.
        if self.next == Some(entry.into()) {
            self.next = next;
        }

        // The entry is now fully unlinked, so we can now take it out safely.
        let entry = unsafe {
            listener
                .get_unchecked_mut()
                .take()
                .unwrap()
                .link
                .into_inner()
        };

        // This State::Created is immediately dropped and exists as a workaround for the absence of
        // loom::cell::Cell::into_inner. The intent is `let mut state = entry.state.into_inner();`
        //
        // refs: https://github.com/tokio-rs/loom/pull/341
        let mut state = entry.state.replace(State::Created);

        // Update the notified count.
        if state.is_notified() {
            self.notified -= 1;

            if propagate {
                let state = mem::replace(&mut state, State::NotifiedTaken);
                if let State::Notified { additional, tag } = state {
                    let tags = {
                        let mut tag = Some(tag);
                        move || tag.take().expect("tag already taken")
                    };
                    self.notify(GenericNotify::new(1, additional, tags));
                }
            }
        }
        self.len -= 1;

        Some(state)
    }

    #[cold]
    fn notify(&mut self, mut notify: impl Notification<Tag = T>) -> usize {
        let mut n = notify.count(Internal::new());
        let is_additional = notify.is_additional(Internal::new());

        if !is_additional {
            if n < self.notified {
                return 0;
            }
            n -= self.notified;
        }

        let original_count = n;
        while n > 0 {
            n -= 1;

            // Notify the next entry.
            match self.next {
                None => return original_count - n - 1,

                Some(e) => {
                    // Get the entry and move the pointer forwards.
                    let entry = unsafe { e.as_ref() };
                    self.next = entry.next.get();

                    // Set the state to `Notified` and notify.
                    let tag = notify.next_tag(Internal::new());
                    if let State::Task(task) = entry.state.replace(State::Notified {
                        additional: is_additional,
                        tag,
                    }) {
                        task.wake();
                    }

                    // Bump the notified count.
                    self.notified += 1;
                }
            }
        }

        original_count - n
    }
}

fn update_notified<T>(slot: &crate::event::sync::atomic::AtomicUsize, list: &Inner<T>) {
    // Update the notified count.
    let notified = if list.notified < list.len {
        list.notified
    } else {
        usize::MAX
    };

    slot.store(notified, Ordering::Release);
}

pub(crate) struct Listener<T> {
    /// The inner link in the linked list.
    ///
    /// # Safety
    ///
    /// This can only be accessed while the central mutex is locked.
    link: UnsafeCell<Link<T>>,

    /// This listener cannot be moved after being pinned.
    _pin: PhantomPinned,
}

struct Link<T> {
    /// The current state of the listener.
    state: Cell<State<T>>,

    /// The previous link in the linked list.
    prev: Cell<Option<NonNull<Link<T>>>>,

    /// The next link in the linked list.
    next: Cell<Option<NonNull<Link<T>>>>,
}

/// Dangerous spinlock for when neither std nor critical-section are enabled.
#[cfg(not(any(feature = "std")))]
mod spinlock {
    use crate::event::sync::atomic::{AtomicBool, Ordering};
    use crate::event::sync::cell::UnsafeCell;

    /// A spinlock.
    pub(super) struct Spinlock<T> {
        /// Is this spinlock locked?
        locked: AtomicBool,

        /// The data contained within the lock.
        data: UnsafeCell<T>,
    }

    impl<T> Spinlock<T> {
        /// Create a new [`Spinlock`].
        #[inline]
        pub(super) fn new(data: T) -> Self {
            Self {
                locked: AtomicBool::new(false),
                data: UnsafeCell::new(data),
            }
        }

        /// Try to acquire a spinlock, run a closure, then release.
        #[inline]
        pub(super) fn try_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
            if self
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                let _guard = CallOnDrop(|| self.locked.store(false, Ordering::Release));
                // SAFETY: We have the lock held.
                Some(f(unsafe { self.data.get().deref_mut() }))
            } else {
                None
            }
        }

        /// Run an operation with the spinlock held.
        #[inline]
        pub(super) fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
            loop {
                if self
                    .locked
                    .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    let _guard = CallOnDrop(|| self.locked.store(false, Ordering::Release));
                    // SAFETY: We have the lock held.
                    return f(unsafe { self.data.get().deref_mut() });
                }

                while self.locked.load(Ordering::Relaxed) {
                    core::hint::spin_loop();
                }
            }
        }
    }

    struct CallOnDrop<F: FnMut()>(F);
    impl<F: FnMut()> Drop for CallOnDrop<F> {
        #[inline]
        fn drop(&mut self) {
            (self.0)();
        }
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/async-notification/event-listener/unit/src/intrusive/tests_471.rs"
));
