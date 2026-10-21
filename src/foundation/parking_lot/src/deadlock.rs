//! \[Experimental\] Deadlock detection
//!
//! This feature is optional and can be enabled via the `deadlock_detection` feature flag.
//!
//! # Example
//!
//! ```
//! #[cfg(feature = "deadlock_detection")]
//! { // only for #[cfg]
//! use std::thread;
//! use std::time::Duration;
//! use parking_lot::deadlock;
//!
//! // Create a background thread which checks for deadlocks every 10s
//! thread::spawn(move || {
//!     loop {
//!         thread::sleep(Duration::from_secs(10));
//!         let deadlocks = deadlock::check_deadlock();
//!         if deadlocks.is_empty() {
//!             continue;
//!         }
//!
//!         println!("{} deadlocks detected", deadlocks.len());
//!         for (i, threads) in deadlocks.iter().enumerate() {
//!             println!("Deadlock #{}", i);
//!             for t in threads {
//!                 println!("Thread Id {:#?}", t.thread_id());
//!                 println!("{:#?}", t.backtrace());
//!             }
//!         }
//!     }
//! });
//! } // only for #[cfg]
//! ```

#[cfg(feature = "deadlock_detection")]
pub use parking_lot_core::deadlock::check_deadlock;
pub(crate) use parking_lot_core::deadlock::{acquire_resource, release_resource};

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/parking_lot/unit/src/deadlock/tests_40.rs"));
