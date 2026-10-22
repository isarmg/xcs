//! Capability-scoped schema values, ECMA patterns, references and offline validation.
#![cfg_attr(
    not(any(feature = "value", feature = "reference", feature = "validator")),
    no_std
)]
extern crate alloc;
#[cfg(feature = "value")]
mod value;
#[cfg(feature = "value")]
pub mod values {
    pub use crate::value::*;
}
#[cfg(feature = "pattern")]
mod pattern;
#[cfg(feature = "pattern")]
pub mod patterns {
    pub use crate::pattern::*;
}
#[cfg(feature = "reference")]
mod reference;
#[cfg(feature = "reference")]
pub mod references {
    pub use crate::reference::*;
}
#[cfg(feature = "validator")]
mod validation;
#[cfg(feature = "validator")]
pub use validation::*;
