//! Tools for working with tasks.

#![warn(missing_docs, unsafe_op_in_unsafe_fn)]

mod spawn;
pub use crate::task_support::spawn::{LocalSpawn, Spawn, SpawnError};

#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
mod arc_wake;
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
pub use crate::task_support::arc_wake::ArcWake;

#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
mod waker;
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
pub use crate::task_support::waker::waker;

#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
mod waker_ref;
#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
pub use crate::task_support::waker_ref::{waker_ref, WakerRef};

mod future_obj;
pub use crate::task_support::future_obj::{FutureObj, LocalFutureObj, UnsafeFutureObj};

mod noop_waker;
pub use crate::task_support::noop_waker::noop_waker;
pub use crate::task_support::noop_waker::noop_waker_ref;
