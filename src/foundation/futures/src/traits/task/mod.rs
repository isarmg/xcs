//! Task notification.

#[macro_use]
mod poll;

#[doc(hidden)]
pub(crate) mod __internal;

#[cfg(all(feature = "utilities", feature = "alloc"))]
pub(crate) use core::task::Waker;
#[cfg(feature = "utilities")]
pub(crate) use core::task::{Context, Poll};
