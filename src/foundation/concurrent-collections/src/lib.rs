//! Concurrent collections, reclamation, and thread utilities.
#![no_std]
#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms),
        allow(dead_code, unused_assignments, unused_variables)
    )
))]
#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "alloc")]
extern crate alloc;
pub mod utilities;
pub mod epoch;
pub mod queue;
pub mod deque;
