// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::locks::raw_fair_mutex::RawFairMutex;

/// A mutual exclusive primitive that is always fair, useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available. The
/// mutex can be statically initialized or created by the `new`
/// constructor. Each mutex has a type parameter which represents the data that
/// it is protecting. The data can only be accessed through the RAII guards
/// returned from `lock` and `try_lock`, which guarantees that the data is only
/// ever accessed when the mutex is locked.
///
/// The regular mutex provided by `parking_lot` uses eventual fairness
/// (after some time it will default to the fair algorithm), but eventual
/// fairness does not provide the same guarantees an always fair method would.
/// Fair mutexes are generally slower, but sometimes needed.
///
/// In a fair mutex the waiters form a queue, and the lock is always granted to
/// the next requester in the queue, in first-in first-out order. This ensures
/// that one thread cannot starve others by quickly re-acquiring the lock after
/// releasing it.
///
/// A fair mutex may not be interesting if threads have different priorities (this is known as
/// priority inversion).
///
/// # Differences from the standard library `Mutex`
///
/// - No poisoning, the lock is released normally on panic.
/// - Only requires 1 byte of space, whereas the standard library boxes the
///   `FairMutex` due to platform limitations.
/// - Can be statically constructed.
/// - Does not require any drop glue when dropped.
/// - Inline fast path for the uncontended case.
/// - Efficient handling of micro-contention using adaptive spinning.
/// - Allows raw locking & unlocking without a guard.
///
/// # Examples
///
/// ```
/// use synchronization::FairMutex;
/// use std::sync::{Arc, mpsc::channel};
/// use std::thread;
///
/// const N: usize = 10;
///
/// // Spawn a few threads to increment a shared variable (non-atomically), and
/// // let the main thread know once all increments are done.
/// //
/// // Here we're using an Arc to share memory among threads, and the data inside
/// // the Arc is protected with a mutex.
/// let data = Arc::new(FairMutex::new(0));
///
/// let (tx, rx) = channel();
/// for _ in 0..10 {
///     let (data, tx) = (Arc::clone(&data), tx.clone());
///     thread::spawn(move || {
///         // The shared state can only be accessed once the lock is held.
///         // Our non-atomic increment is safe because we're the only thread
///         // which can access the shared state when the lock is held.
///         let mut data = data.lock();
///         *data += 1;
///         if *data == N {
///             tx.send(()).unwrap();
///         }
///         // the lock is unlocked here when `data` goes out of scope.
///     });
/// }
///
/// rx.recv().unwrap();
/// ```
pub type FairMutex<T> = crate::api::Mutex<RawFairMutex, T>;

/// Creates a new fair mutex in an unlocked state ready for use.
///
/// This allows creating a fair mutex in a constant context on stable Rust.
pub const fn const_fair_mutex<T>(val: T) -> FairMutex<T> {
    FairMutex::const_new(<RawFairMutex as crate::api::RawMutex>::INIT, val)
}

/// An RAII implementation of a "scoped lock" of a mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// `Deref` and `DerefMut` implementations.
pub type FairMutexGuard<'a, T> = crate::api::MutexGuard<'a, RawFairMutex, T>;

/// An RAII mutex guard returned by `FairMutexGuard::map`, which can point to a
/// subfield of the protected data.
///
/// The main difference between `MappedFairMutexGuard` and `FairMutexGuard` is that the
/// former doesn't support temporarily unlocking and re-locking, since that
/// could introduce soundness issues if the locked object is modified by another
/// thread.
pub type MappedFairMutexGuard<'a, T> = crate::api::MappedMutexGuard<'a, RawFairMutex, T>;

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/synchronization/locks/unit/src/fair_mutex/tests_102.rs"
));
