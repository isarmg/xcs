//! A ticket-based mutex.
//!
//! Waiting threads take a 'ticket' from the lock in the order they arrive and gain access to the lock when their
//! ticket is next in the queue. Best-case latency is slightly worse than a regular spinning mutex, but worse-case
//! latency is infinitely better. Waiting threads simply need to wait for all threads that come before them in the
//! queue to finish.

use crate::{
    atomic::{AtomicUsize, Ordering},
    RelaxStrategy, Spin,
};
use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A spin-based [ticket lock](https://en.wikipedia.org/wiki/Ticket_lock) providing mutually exclusive access to data.
///
/// A ticket lock is analogous to a queue management system for lock requests. When a thread tries to take a lock, it
/// is assigned a 'ticket'. It then spins until its ticket becomes next in line. When the lock guard is released, the
/// next ticket will be processed.
///
/// Ticket locks significantly reduce the worse-case performance of locking at the cost of slightly higher average-time
/// overhead.
///
/// # Example
///
/// ```
/// use spin;
///
/// let lock = spin::mutex::TicketMutex::<_>::new(0);
///
/// // Modify the data
/// *lock.lock() = 2;
///
/// // Read the data
/// let answer = *lock.lock();
/// assert_eq!(answer, 2);
/// ```
///
/// # Thread safety example
///
/// ```
/// use spin;
/// use std::sync::{Arc, Barrier};
///
/// let thread_count = 1000;
/// let spin_mutex = Arc::new(spin::mutex::TicketMutex::<_>::new(0));
///
/// // We use a barrier to ensure the readout happens after all writing
/// let barrier = Arc::new(Barrier::new(thread_count + 1));
///
/// for _ in (0..thread_count) {
///     let my_barrier = barrier.clone();
///     let my_lock = spin_mutex.clone();
///     std::thread::spawn(move || {
///         let mut guard = my_lock.lock();
///         *guard += 1;
///
///         // Release the lock to prevent a deadlock
///         drop(guard);
///         my_barrier.wait();
///     });
/// }
///
/// barrier.wait();
///
/// let answer = { *spin_mutex.lock() };
/// assert_eq!(answer, thread_count);
/// ```
pub struct TicketMutex<T: ?Sized, R = Spin> {
    phantom: PhantomData<R>,
    next_ticket: AtomicUsize,
    next_serving: AtomicUsize,
    data: UnsafeCell<T>,
}

/// A guard that protects some data.
///
/// When the guard is dropped, the next ticket will be processed.
pub struct TicketMutexGuard<'a, T: ?Sized + 'a> {
    next_serving: &'a AtomicUsize,
    ticket: usize,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send, R> Sync for TicketMutex<T, R> {}
unsafe impl<T: ?Sized + Send, R> Send for TicketMutex<T, R> {}

impl<T, R> TicketMutex<T, R> {
    /// Creates a new [`TicketMutex`] wrapping the supplied data.
    ///
    /// # Example
    ///
    /// ```
    /// use spin::mutex::TicketMutex;
    ///
    /// static MUTEX: TicketMutex<()> = TicketMutex::<_>::new(());
    ///
    /// fn demo() {
    ///     let lock = MUTEX.lock();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self {
            phantom: PhantomData,
            next_ticket: AtomicUsize::new(0),
            next_serving: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this [`TicketMutex`] and unwraps the underlying data.
    ///
    /// # Example
    ///
    /// ```
    /// let lock = spin::mutex::TicketMutex::<_>::new(42);
    /// assert_eq!(42, lock.into_inner());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
    /// Returns a mutable pointer to the underying data.
    ///
    /// This is mostly meant to be used for applications which require manual unlocking, but where
    /// storing both the lock and the pointer to the inner data gets inefficient.
    ///
    /// # Example
    /// ```
    /// let lock = spin::mutex::SpinMutex::<_>::new(42);
    ///
    /// unsafe {
    ///     core::mem::forget(lock.lock());
    ///
    ///     assert_eq!(lock.as_mut_ptr().read(), 42);
    ///     lock.as_mut_ptr().write(58);
    ///
    ///     lock.force_unlock();
    /// }
    ///
    /// assert_eq!(*lock.lock(), 58);
    ///
    /// ```
    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized + fmt::Debug, R> fmt::Debug for TicketMutex<T, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized, R: RelaxStrategy> TicketMutex<T, R> {
    /// Locks the [`TicketMutex`] and returns a guard that permits access to the inner data.
    ///
    /// The returned data may be dereferenced for data access
    /// and the lock will be dropped when the guard falls out of scope.
    ///
    /// ```
    /// let lock = spin::mutex::TicketMutex::<_>::new(0);
    /// {
    ///     let mut data = lock.lock();
    ///     // The lock is now locked and the data can be accessed
    ///     *data += 1;
    ///     // The lock is implicitly dropped at the end of the scope
    /// }
    /// ```
    #[inline(always)]
    pub fn lock(&self) -> TicketMutexGuard<T> {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        while self.next_serving.load(Ordering::Acquire) != ticket {
            R::relax();
        }

        TicketMutexGuard {
            next_serving: &self.next_serving,
            ticket,
            // Safety
            // We know that we are the next ticket to be served,
            // so there's no other thread accessing the data.
            //
            // Every other thread has another ticket number so it's
            // definitely stuck in the spin loop above.
            data: unsafe { &mut *self.data.get() },
        }
    }
}

impl<T: ?Sized, R> TicketMutex<T, R> {
    /// Returns `true` if the lock is currently held.
    ///
    /// # Safety
    ///
    /// This function provides no synchronization guarantees and so its result should be considered 'out of date'
    /// the instant it is called. Do not use it for synchronization purposes. However, it may be useful as a heuristic.
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        let ticket = self.next_ticket.load(Ordering::Relaxed);
        self.next_serving.load(Ordering::Relaxed) != ticket
    }

    /// Force unlock this [`TicketMutex`], by serving the next ticket.
    ///
    /// # Safety
    ///
    /// This is *extremely* unsafe if the lock is not held by the current
    /// thread. However, this can be useful in some instances for exposing the
    /// lock to FFI that doesn't know how to deal with RAII.
    #[inline(always)]
    pub unsafe fn force_unlock(&self) {
        self.next_serving.fetch_add(1, Ordering::Release);
    }

    /// Try to lock this [`TicketMutex`], returning a lock guard if successful.
    ///
    /// # Example
    ///
    /// ```
    /// let lock = spin::mutex::TicketMutex::<_>::new(42);
    ///
    /// let maybe_guard = lock.try_lock();
    /// assert!(maybe_guard.is_some());
    ///
    /// // `maybe_guard` is still held, so the second call fails
    /// let maybe_guard2 = lock.try_lock();
    /// assert!(maybe_guard2.is_none());
    /// ```
    #[inline(always)]
    pub fn try_lock(&self) -> Option<TicketMutexGuard<T>> {
        let ticket = self
            .next_ticket
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ticket| {
                if self.next_serving.load(Ordering::Acquire) == ticket {
                    Some(ticket + 1)
                } else {
                    None
                }
            });

        ticket.ok().map(|ticket| TicketMutexGuard {
            next_serving: &self.next_serving,
            ticket,
            // Safety
            // We have a ticket that is equal to the next_serving ticket, so we know:
            // - that no other thread can have the same ticket id as this thread
            // - that we are the next one to be served so we have exclusive access to the data
            data: unsafe { &mut *self.data.get() },
        })
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`TicketMutex`] mutably, and a mutable reference is guaranteed to be exclusive in
    /// Rust, no actual locking needs to take place -- the mutable borrow statically guarantees no locks exist. As
    /// such, this is a 'zero-cost' operation.
    ///
    /// # Example
    ///
    /// ```
    /// let mut lock = spin::mutex::TicketMutex::<_>::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.lock(), 10);
    /// ```
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // Safety:
        // We know that there are no other references to `self`,
        // so it's safe to return a exclusive reference to the data.
        unsafe { &mut *self.data.get() }
    }
}

impl<T: ?Sized + Default, R> Default for TicketMutex<T, R> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T, R> From<T> for TicketMutex<T, R> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized> TicketMutexGuard<'a, T> {
    /// Leak the lock guard, yielding a mutable reference to the underlying data.
    ///
    /// Note that this function will permanently lock the original [`TicketMutex`].
    ///
    /// ```
    /// let mylock = spin::mutex::TicketMutex::<_>::new(0);
    ///
    /// let data: &mut i32 = spin::mutex::TicketMutexGuard::leak(mylock.lock());
    ///
    /// *data = 1;
    /// assert_eq!(*data, 1);
    /// ```
    #[inline(always)]
    pub fn leak(this: Self) -> &'a mut T {
        let data = this.data as *mut _; // Keep it in pointer form temporarily to avoid double-aliasing
        core::mem::forget(this);
        unsafe { &mut *data }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for TicketMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for TicketMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for TicketMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for TicketMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for TicketMutexGuard<'a, T> {
    fn drop(&mut self) {
        let new_ticket = self.ticket + 1;
        self.next_serving.store(new_ticket, Ordering::Release);
    }
}

#[cfg(feature = "lock_api")]
unsafe impl<R: RelaxStrategy> synchronization::api::RawMutex for TicketMutex<(), R> {
    type GuardMarker = synchronization::api::GuardSend;

    const INIT: Self = Self::new(());

    fn lock(&self) {
        // Prevent guard destructor running
        core::mem::forget(Self::lock(self));
    }

    fn try_lock(&self) -> bool {
        // Prevent guard destructor running
        Self::try_lock(self).map(core::mem::forget).is_some()
    }

    unsafe fn unlock(&self) {
        self.force_unlock();
    }

    fn is_locked(&self) -> bool {
        Self::is_locked(self)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/spin/unit/src/mutex/ticket/tests_379.rs"));
