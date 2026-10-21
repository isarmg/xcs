//! Project-owned synchronization: generic lock contracts, parking and concrete locks.
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(rust_2018_idioms)]
#[macro_use]
extern crate scopeguard;
#[cfg(feature = "arc_lock")]
extern crate alloc;
/// Generic, no-std lock contracts and typed guards.
pub mod api;
#[cfg(feature = "locks")]
mod locks;
/// Low-level thread parking and wakeup operations.
#[cfg(feature = "parking")]
pub mod parking;
#[cfg(feature = "locks")]
pub use locks::*;
