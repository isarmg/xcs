// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::mutex::MutexGuard;
use crate::raw_mutex::{RawMutex, TOKEN_HANDOFF, TOKEN_NORMAL};
use crate::{deadlock, util};
use core::{
    fmt, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};
use lock_api::RawMutex as RawMutex_;
use parking_lot_core::{self, ParkResult, RequeueOp, UnparkResult, DEFAULT_PARK_TOKEN};
use std::ops::DerefMut;
use std::time::{Duration, Instant};

/// A type indicating whether a timed wait on a condition variable returned
/// due to a time out or not.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    /// Returns whether the wait was known to have timed out.
    #[inline]
    pub fn timed_out(self) -> bool {
        self.0
    }
}

/// A Condition Variable
///
/// Condition variables represent the ability to block a thread such that it
/// consumes no CPU time while waiting for an event to occur. Condition
/// variables are typically associated with a boolean predicate (a condition)
/// and a mutex. The predicate is always verified inside of the mutex before
/// determining that thread must block.
///
/// Note that this module places one additional restriction over the system
/// condition variables: each condvar can be used with only one mutex at a
/// time. Any attempt to use multiple mutexes on the same condition variable
/// simultaneously will result in a runtime panic. However it is possible to
/// switch to a different mutex if there are no threads currently waiting on
/// the condition variable.
///
/// # Differences from the standard library `Condvar`
///
/// - No spurious wakeups: A wait will only return a non-timeout result if it
///   was woken up by `notify_one` or `notify_all`.
/// - `Condvar::notify_all` will only wake up a single thread, the rest are
///   requeued to wait for the `Mutex` to be unlocked by the thread that was
///   woken up.
/// - Only requires 1 word of space, whereas the standard library boxes the
///   `Condvar` due to platform limitations.
/// - Can be statically constructed.
/// - Does not require any drop glue when dropped.
/// - Inline fast path for the uncontended case.
///
/// # Examples
///
/// ```
/// use parking_lot::{Mutex, Condvar};
/// use std::sync::Arc;
/// use std::thread;
///
/// let pair = Arc::new((Mutex::new(false), Condvar::new()));
/// let pair2 = pair.clone();
///
/// // Inside of our lock, spawn a new thread, and then wait for it to start
/// thread::spawn(move|| {
///     let &(ref lock, ref cvar) = &*pair2;
///     let mut started = lock.lock();
///     *started = true;
///     cvar.notify_one();
/// });
///
/// // wait for the thread to start up
/// let &(ref lock, ref cvar) = &*pair;
/// let mut started = lock.lock();
/// if !*started {
///     cvar.wait(&mut started);
/// }
/// // Note that we used an if instead of a while loop above. This is only
/// // possible because parking_lot's Condvar will never spuriously wake up.
/// // This means that wait() will only return after notify_one or notify_all is
/// // called.
/// ```
pub struct Condvar {
    state: AtomicPtr<RawMutex>,
}

impl Condvar {
    /// Creates a new condition variable which is ready to be waited on and
    /// notified.
    #[inline]
    pub const fn new() -> Condvar {
        Condvar {
            state: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Wakes up one blocked thread on this condvar.
    ///
    /// Returns whether a thread was woken up.
    ///
    /// If there is a blocked thread on this condition variable, then it will
    /// be woken up from its call to `wait` or `wait_timeout`. Calls to
    /// `notify_one` are not buffered in any way.
    ///
    /// To wake up all threads, see `notify_all()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use parking_lot::Condvar;
    ///
    /// let condvar = Condvar::new();
    ///
    /// // do something with condvar, share it with other threads
    ///
    /// if !condvar.notify_one() {
    ///     println!("Nobody was listening for this.");
    /// }
    /// ```
    #[inline]
    pub fn notify_one(&self) -> bool {
        // Nothing to do if there are no waiting threads
        let state = self.state.load(Ordering::Relaxed);
        if state.is_null() {
            return false;
        }

        self.notify_one_slow(state)
    }

    #[cold]
    fn notify_one_slow(&self, mutex: *mut RawMutex) -> bool {
        // Unpark one thread and requeue the rest onto the mutex
        let from = self as *const _ as usize;
        let to = mutex as usize;
        let validate = || {
            // Make sure that our atomic state still points to the same
            // mutex. If not then it means that all threads on the current
            // mutex were woken up and a new waiting thread switched to a
            // different mutex. In that case we can get away with doing
            // nothing.
            if self.state.load(Ordering::Relaxed) != mutex {
                return RequeueOp::Abort;
            }

            // Unpark one thread if the mutex is unlocked, otherwise just
            // requeue everything to the mutex. This is safe to do here
            // since unlocking the mutex when the parked bit is set requires
            // locking the queue. There is the possibility of a race if the
            // mutex gets locked after we check, but that doesn't matter in
            // this case.
            if unsafe { (*mutex).mark_parked_if_locked() } {
                RequeueOp::RequeueOne
            } else {
                RequeueOp::UnparkOne
            }
        };
        let callback = |_op, result: UnparkResult| {
            // Clear our state if there are no more waiting threads
            if !result.have_more_threads {
                self.state.store(ptr::null_mut(), Ordering::Relaxed);
            }
            TOKEN_NORMAL
        };
        let res = unsafe { parking_lot_core::unpark_requeue(from, to, validate, callback) };

        res.unparked_threads + res.requeued_threads != 0
    }

    /// Wakes up all blocked threads on this condvar.
    ///
    /// Returns the number of threads woken up.
    ///
    /// This method will ensure that any current waiters on the condition
    /// variable are awoken. Calls to `notify_all()` are not buffered in any
    /// way.
    ///
    /// To wake up only one thread, see `notify_one()`.
    #[inline]
    pub fn notify_all(&self) -> usize {
        // Nothing to do if there are no waiting threads
        let state = self.state.load(Ordering::Relaxed);
        if state.is_null() {
            return 0;
        }

        self.notify_all_slow(state)
    }

    #[cold]
    fn notify_all_slow(&self, mutex: *mut RawMutex) -> usize {
        // Unpark one thread and requeue the rest onto the mutex
        let from = self as *const _ as usize;
        let to = mutex as usize;
        let validate = || {
            // Make sure that our atomic state still points to the same
            // mutex. If not then it means that all threads on the current
            // mutex were woken up and a new waiting thread switched to a
            // different mutex. In that case we can get away with doing
            // nothing.
            if self.state.load(Ordering::Relaxed) != mutex {
                return RequeueOp::Abort;
            }

            // Clear our state since we are going to unpark or requeue all
            // threads.
            self.state.store(ptr::null_mut(), Ordering::Relaxed);

            // Unpark one thread if the mutex is unlocked, otherwise just
            // requeue everything to the mutex. This is safe to do here
            // since unlocking the mutex when the parked bit is set requires
            // locking the queue. There is the possibility of a race if the
            // mutex gets locked after we check, but that doesn't matter in
            // this case.
            if unsafe { (*mutex).mark_parked_if_locked() } {
                RequeueOp::RequeueAll
            } else {
                RequeueOp::UnparkOneRequeueRest
            }
        };
        let callback = |op, result: UnparkResult| {
            // If we requeued threads to the mutex, mark it as having
            // parked threads. The RequeueAll case is already handled above.
            if op == RequeueOp::UnparkOneRequeueRest && result.requeued_threads != 0 {
                unsafe { (*mutex).mark_parked() };
            }
            TOKEN_NORMAL
        };
        let res = unsafe { parking_lot_core::unpark_requeue(from, to, validate, callback) };

        res.unparked_threads + res.requeued_threads
    }

    /// Blocks the current thread until this condition variable receives a
    /// notification.
    ///
    /// This function will atomically unlock the mutex specified (represented by
    /// `mutex_guard`) and block the current thread. This means that any calls
    /// to `notify_*()` which happen logically after the mutex is unlocked are
    /// candidates to wake this thread up. When this function call returns, the
    /// lock specified will have been re-acquired.
    ///
    /// # Panics
    ///
    /// This function will panic if another thread is waiting on the `Condvar`
    /// with a different `Mutex` object.
    #[inline]
    pub fn wait<T: ?Sized>(&self, mutex_guard: &mut MutexGuard<'_, T>) {
        self.wait_until_internal(unsafe { MutexGuard::mutex(mutex_guard).raw() }, None);
    }

    /// Waits on this condition variable for a notification, timing out after
    /// the specified time instant.
    ///
    /// The semantics of this function are equivalent to `wait()` except that
    /// the thread will be blocked roughly until `timeout` is reached. This
    /// method should not be used for precise timing due to anomalies such as
    /// preemption or platform differences that may not cause the maximum
    /// amount of time waited to be precisely `timeout`.
    ///
    /// Note that the best effort is made to ensure that the time waited is
    /// measured with a monotonic clock, and not affected by the changes made to
    /// the system time.
    ///
    /// The returned `WaitTimeoutResult` value indicates if the timeout is
    /// known to have elapsed.
    ///
    /// Like `wait`, the lock specified will be re-acquired when this function
    /// returns, regardless of whether the timeout elapsed or not.
    ///
    /// # Panics
    ///
    /// This function will panic if another thread is waiting on the `Condvar`
    /// with a different `Mutex` object.
    #[inline]
    pub fn wait_until<T: ?Sized>(
        &self,
        mutex_guard: &mut MutexGuard<'_, T>,
        timeout: Instant,
    ) -> WaitTimeoutResult {
        self.wait_until_internal(
            unsafe { MutexGuard::mutex(mutex_guard).raw() },
            Some(timeout),
        )
    }

    // This is a non-generic function to reduce the monomorphization cost of
    // using `wait_until`.
    fn wait_until_internal(&self, mutex: &RawMutex, timeout: Option<Instant>) -> WaitTimeoutResult {
        let result;
        let mut bad_mutex = false;
        let mut requeued = false;
        {
            let addr = self as *const _ as usize;
            let lock_addr = mutex as *const _ as *mut _;
            let validate = || {
                // Ensure we don't use two different mutexes with the same
                // Condvar at the same time. This is done while locked to
                // avoid races with notify_one
                let state = self.state.load(Ordering::Relaxed);
                if state.is_null() {
                    self.state.store(lock_addr, Ordering::Relaxed);
                } else if state != lock_addr {
                    bad_mutex = true;
                    return false;
                }
                true
            };
            let before_sleep = || {
                // Unlock the mutex before sleeping...
                unsafe { mutex.unlock() };
            };
            let timed_out = |k, was_last_thread| {
                // If we were requeued to a mutex, then we did not time out.
                // We'll just park ourselves on the mutex again when we try
                // to lock it later.
                requeued = k != addr;

                // If we were the last thread on the queue then we need to
                // clear our state. This is normally done by the
                // notify_{one,all} functions when not timing out.
                if !requeued && was_last_thread {
                    self.state.store(ptr::null_mut(), Ordering::Relaxed);
                }
            };
            result = unsafe {
                parking_lot_core::park(
                    addr,
                    validate,
                    before_sleep,
                    timed_out,
                    DEFAULT_PARK_TOKEN,
                    timeout,
                )
            };
        }

        // Panic if we tried to use multiple mutexes with a Condvar. Note
        // that at this point the MutexGuard is still locked. It will be
        // unlocked by the unwinding logic.
        if bad_mutex {
            panic!("attempted to use a condition variable with more than one mutex");
        }

        // ... and re-lock it once we are done sleeping
        if result == ParkResult::Unparked(TOKEN_HANDOFF) {
            unsafe { deadlock::acquire_resource(mutex as *const _ as usize) };
        } else {
            mutex.lock();
        }

        WaitTimeoutResult(!(result.is_unparked() || requeued))
    }

    /// Waits on this condition variable for a notification, timing out after a
    /// specified duration.
    ///
    /// The semantics of this function are equivalent to `wait()` except that
    /// the thread will be blocked for roughly no longer than `timeout`. This
    /// method should not be used for precise timing due to anomalies such as
    /// preemption or platform differences that may not cause the maximum
    /// amount of time waited to be precisely `timeout`.
    ///
    /// Note that the best effort is made to ensure that the time waited is
    /// measured with a monotonic clock, and not affected by the changes made to
    /// the system time.
    ///
    /// The returned `WaitTimeoutResult` value indicates if the timeout is
    /// known to have elapsed.
    ///
    /// Like `wait`, the lock specified will be re-acquired when this function
    /// returns, regardless of whether the timeout elapsed or not.
    #[inline]
    pub fn wait_for<T: ?Sized>(
        &self,
        mutex_guard: &mut MutexGuard<'_, T>,
        timeout: Duration,
    ) -> WaitTimeoutResult {
        let deadline = util::to_deadline(timeout);
        self.wait_until_internal(unsafe { MutexGuard::mutex(mutex_guard).raw() }, deadline)
    }

    #[inline]
    fn wait_while_until_internal<T, F>(
        &self,
        mutex_guard: &mut MutexGuard<'_, T>,
        mut condition: F,
        timeout: Option<Instant>,
    ) -> WaitTimeoutResult
    where
        T: ?Sized,
        F: FnMut(&mut T) -> bool,
    {
        let mut result = WaitTimeoutResult(false);

        while !result.timed_out() && condition(mutex_guard.deref_mut()) {
            result =
                self.wait_until_internal(unsafe { MutexGuard::mutex(mutex_guard).raw() }, timeout);
        }

        result
    }
    /// Blocks the current thread until this condition variable receives a
    /// notification. If the provided condition evaluates to `false`, then the
    /// thread is no longer blocked and the operation is completed. If the
    /// condition evaluates to `true`, then the thread is blocked again and
    /// waits for another notification before repeating this process.
    ///
    /// This function will atomically unlock the mutex specified (represented by
    /// `mutex_guard`) and block the current thread. This means that any calls
    /// to `notify_*()` which happen logically after the mutex is unlocked are
    /// candidates to wake this thread up. When this function call returns, the
    /// lock specified will have been re-acquired.
    ///
    /// # Panics
    ///
    /// This function will panic if another thread is waiting on the `Condvar`
    /// with a different `Mutex` object.
    #[inline]
    pub fn wait_while<T, F>(&self, mutex_guard: &mut MutexGuard<'_, T>, condition: F)
    where
        T: ?Sized,
        F: FnMut(&mut T) -> bool,
    {
        self.wait_while_until_internal(mutex_guard, condition, None);
    }

    /// Waits on this condition variable for a notification, timing out after
    /// the specified time instant. If the provided condition evaluates to
    /// `false`, then the thread is no longer blocked and the operation is
    /// completed. If the condition evaluates to `true`, then the thread is
    /// blocked again and waits for another notification before repeating
    /// this process.
    ///
    /// The semantics of this function are equivalent to `wait()` except that
    /// the thread will be blocked roughly until `timeout` is reached. This
    /// method should not be used for precise timing due to anomalies such as
    /// preemption or platform differences that may not cause the maximum
    /// amount of time waited to be precisely `timeout`.
    ///
    /// Note that the best effort is made to ensure that the time waited is
    /// measured with a monotonic clock, and not affected by the changes made to
    /// the system time.
    ///
    /// The returned `WaitTimeoutResult` value indicates if the timeout is
    /// known to have elapsed.
    ///
    /// Like `wait`, the lock specified will be re-acquired when this function
    /// returns, regardless of whether the timeout elapsed or not.
    ///
    /// # Panics
    ///
    /// This function will panic if another thread is waiting on the `Condvar`
    /// with a different `Mutex` object.
    #[inline]
    pub fn wait_while_until<T, F>(
        &self,
        mutex_guard: &mut MutexGuard<'_, T>,
        condition: F,
        timeout: Instant,
    ) -> WaitTimeoutResult
    where
        T: ?Sized,
        F: FnMut(&mut T) -> bool,
    {
        self.wait_while_until_internal(mutex_guard, condition, Some(timeout))
    }

    /// Waits on this condition variable for a notification, timing out after a
    /// specified duration. If the provided condition evaluates to `false`,
    /// then the thread is no longer blocked and the operation is completed.
    /// If the condition evaluates to `true`, then the thread is blocked again
    /// and waits for another notification before repeating this process.
    ///
    /// The semantics of this function are equivalent to `wait()` except that
    /// the thread will be blocked for roughly no longer than `timeout`. This
    /// method should not be used for precise timing due to anomalies such as
    /// preemption or platform differences that may not cause the maximum
    /// amount of time waited to be precisely `timeout`.
    ///
    /// Note that the best effort is made to ensure that the time waited is
    /// measured with a monotonic clock, and not affected by the changes made to
    /// the system time.
    ///
    /// The returned `WaitTimeoutResult` value indicates if the timeout is
    /// known to have elapsed.
    ///
    /// Like `wait`, the lock specified will be re-acquired when this function
    /// returns, regardless of whether the timeout elapsed or not.
    #[inline]
    pub fn wait_while_for<T: ?Sized, F>(
        &self,
        mutex_guard: &mut MutexGuard<'_, T>,
        condition: F,
        timeout: Duration,
    ) -> WaitTimeoutResult
    where
        F: FnMut(&mut T) -> bool,
    {
        let deadline = util::to_deadline(timeout);
        self.wait_while_until_internal(mutex_guard, condition, deadline)
    }
}

impl Default for Condvar {
    #[inline]
    fn default() -> Condvar {
        Condvar::new()
    }
}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Condvar { .. }")
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/parking_lot/unit/src/condvar/tests_525.rs"));

/// This module contains an integration test that is heavily inspired from WebKit's own integration
/// tests for it's own Condvar.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/parking_lot/unit/src/condvar/webkit_queue_test_916.rs"));
