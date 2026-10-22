//! Core items for `time`.
//!
//! This crate is an implementation detail of `time` and should not be relied upon directly.

#![doc(test(attr(deny(warnings))))]

mod hint;
pub mod unit;
pub(crate) mod util;
