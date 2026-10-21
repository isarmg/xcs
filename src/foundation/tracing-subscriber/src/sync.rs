//! Abstracts over sync primitive implementations.
//!
//! Optionally, we allow the Rust standard library's `RwLock` to be replaced
//! with the `synchronization` crate's implementation. This may provide improved
//! performance in some cases. However, the `synchronization` dependency is an
//! opt-in feature flag. Because `synchronization::RwLock` has a slightly different
//! API than `std::sync::RwLock` (it does not support poisoning on panics), we
//! wrap it with a type that provides the same method signatures. This allows us
//! to transparently swap `synchronization` in without changing code at the callsite.
#[allow(unused_imports)] // may be used later;
pub(crate) use std::sync::{LockResult, PoisonError, TryLockResult};

#[cfg(not(feature = "parking"))]
pub(crate) use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "parking")]
pub(crate) use self::parking_impl::*;

#[cfg(feature = "parking")]
mod parking_impl {
    pub(crate) use synchronization::{RwLockReadGuard, RwLockWriteGuard};
    use std::sync::{LockResult, TryLockError, TryLockResult};

    #[derive(Debug)]
    pub(crate) struct RwLock<T> {
        inner: synchronization::RwLock<T>,
    }

    impl<T> RwLock<T> {
        pub(crate) fn new(val: T) -> Self {
            Self {
                inner: synchronization::RwLock::new(val),
            }
        }

        #[inline]
        pub(crate) fn get_mut(&mut self) -> LockResult<&mut T> {
            Ok(self.inner.get_mut())
        }

        #[inline]
        pub(crate) fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
            Ok(self.inner.read())
        }

        #[inline]
        #[allow(dead_code)] // may be used later;
        pub(crate) fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
            self.inner.try_read().ok_or(TryLockError::WouldBlock)
        }

        #[inline]
        pub(crate) fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
            Ok(self.inner.write())
        }
    }

    impl<T: Default> Default for RwLock<T> {
        fn default() -> Self {
            RwLock {
                inner: synchronization::RwLock::default(),
            }
        }
    }
}
