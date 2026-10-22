//! # Retained upstream logging examples
//!
//! Adapted only for the logging crate name from env_logger 0.11.11 README.md.
//! Original source and MIT/Apache-2.0 attribution remain under docs/upstream and compliance.
//!
//! ```rust
//! use log::info;
//!
//! fn main() {
//!     logging::init();
//!
//!     info!("starting up");
//!
//!     // ...
//! }
//! ```
//!
//! ```rust
//! use log::info;
//!
//! fn add_one(num: i32) -> i32 {
//!     info!("add_one called with {}", num);
//!     num + 1
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!
//!     fn init() {
//!         let _ = logging::builder().is_test(true).try_init();
//!     }
//!
//!     #[test]
//!     fn it_adds_one() {
//!         init();
//!
//!         info!("can log from the test too");
//!         assert_eq!(3, add_one(2));
//!     }
//!
//!     #[test]
//!     fn it_handles_negative_numbers() {
//!         init();
//!
//!         info!("logging from another test");
//!         assert_eq!(-7, add_one(-8));
//!     }
//! }
//! ```
//!
//! ```rust
//! use std::env;
//! use logging::{Builder, Target};
//!
//! let mut builder = Builder::from_default_env();
//! builder.target(Target::Stdout);
//!
//! builder.init();
//! ```
//!
