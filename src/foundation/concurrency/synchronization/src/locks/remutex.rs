// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::api::GetThreadId;
use crate::locks::raw_mutex::RawMutex;
use core::num::NonZeroUsize;

/// Implementation of the `GetThreadId` trait for `synchronization::api::ReentrantMutex`.
pub struct RawThreadId;

unsafe impl GetThreadId for RawThreadId {
    const INIT: RawThreadId = RawThreadId;

    fn nonzero_thread_id(&self) -> NonZeroUsize {
        // The address of a thread-local variable is guaranteed to be unique to the
        // current thread, and is also guaranteed to be non-zero. The variable has to have a
        // non-zero size to guarantee it has a unique address for each thread.
        thread_local!(static KEY: u8 = 0);
        KEY.with(|x| {
            NonZeroUsize::new(x as *const _ as usize)
                .expect("thread-local variable address is null")
        })
    }
}

/// A mutex which can be recursively locked by a single thread.
///
/// This type is identical to `Mutex` except for the following points:
///
/// - Locking multiple times from the same thread will work correctly instead of
///   deadlocking.
/// - `ReentrantMutexGuard` does not give mutable references to the locked data.
///   Use a `RefCell` if you need this.
///
/// See [`Mutex`](synchronization::Mutex) for more details about the underlying mutex
/// primitive.
pub type ReentrantMutex<T> = crate::api::ReentrantMutex<RawMutex, RawThreadId, T>;

/// Creates a new reentrant mutex in an unlocked state ready for use.
///
/// This allows creating a reentrant mutex in a constant context on stable Rust.
pub const fn const_reentrant_mutex<T>(val: T) -> ReentrantMutex<T> {
    ReentrantMutex::const_new(
        <RawMutex as crate::api::RawMutex>::INIT,
        <RawThreadId as crate::api::GetThreadId>::INIT,
        val,
    )
}

/// An RAII implementation of a "scoped lock" of a reentrant mutex. When this structure
/// is dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// `Deref` implementation.
pub type ReentrantMutexGuard<'a, T> = crate::api::ReentrantMutexGuard<'a, RawMutex, RawThreadId, T>;

/// An RAII mutex guard returned by `ReentrantMutexGuard::map`, which can point to a
/// subfield of the protected data.
///
/// The main difference between `MappedReentrantMutexGuard` and `ReentrantMutexGuard` is that the
/// former doesn't support temporarily unlocking and re-locking, since that
/// could introduce soundness issues if the locked object is modified by another
/// thread.
pub type MappedReentrantMutexGuard<'a, T> =
    crate::api::MappedReentrantMutexGuard<'a, RawMutex, RawThreadId, T>;

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/synchronization/locks/unit/src/remutex/tests_71.rs"
));
