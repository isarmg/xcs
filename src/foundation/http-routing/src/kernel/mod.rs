//! Shared HTTP extraction, body and response kernel.
//!
//! Libraries authors that want to provide [`FromRequest`] or [`IntoResponse`] implementations
//! use the enclosing `http-routing` package with default features disabled.
//!
//! [`FromRequest`]: crate::kernel::extract::FromRequest
//! [`IntoResponse`]: crate::kernel::response::IntoResponse
//! [`axum`]: https://crates.io/crates/axum
//! [`axum-core`]: http://crates.io/crates/axum-core

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::dbg_macro))]

#[macro_use]
pub(crate) mod macros;
#[doc(hidden)] // macro helpers
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use tracing;
}

mod error;
mod ext_traits;
pub use self::error::Error;

pub mod body;
pub mod extract;
pub mod response;

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub use self::ext_traits::{request::RequestExt, request_parts::RequestPartsExt};

#[cfg(test)]
use axum_macros::__private_axum_test as test;
