//! Core traits and types for asynchronous operations in Rust.

#![warn(missing_docs, /* unsafe_op_in_unsafe_fn */)] // unsafe_op_in_unsafe_fn requires Rust 1.52

pub(crate) mod future;
#[doc(no_inline)]
#[cfg(any(feature = "utilities", all(feature = "channel", feature = "std")))]
pub(crate) use self::future::FusedFuture;
#[cfg(feature = "utilities")]
pub(crate) use self::future::Future;

pub(crate) mod stream;
#[cfg(all(feature = "utilities", feature = "alloc"))]
pub(crate) use self::stream::FusedStream;
#[doc(no_inline)]
#[cfg(feature = "utilities")]
pub(crate) use self::stream::Stream;

#[macro_use]
pub(crate) mod task;

#[cfg(feature = "utilities")]
pub(crate) use crate::ready;
