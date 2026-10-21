//! URL syntax, domain normalization and opt-in form encoding.
#![no_std]
#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "parser", macro_use)]
extern crate alloc;
pub mod percent;
#[cfg(feature = "form")]
pub mod form;
#[cfg(feature = "domain")]
pub mod domain;
#[cfg(feature = "domain")]
mod unicode_backend;
#[cfg(feature = "parser")]
pub mod address;
