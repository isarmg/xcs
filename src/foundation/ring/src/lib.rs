// Copyright 2015-2016 Brian Smith.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
// SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
// OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
// CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! # Supported Target
//!
//! This internal component supports only 64-bit little-endian Linux x86_64.
//! The project's browser Web build does not depend on this component.
//!
//! # Feature Flags
//!
//! <table>
//! <tr><th>Feature
//!     <th>Description
//! <tr><td><code>alloc (default)</code>
//!     <td>Enable features that require use of the heap, RSA in particular.
//! <tr><td><code>std</code>
//!     <td>Enable features that use libstd, in particular
//!         <code>std::error::Error</code> integration. Implies `alloc`.
//! </table>

// When running mk/package.sh, don't actually build any code.
#![allow(
    clippy::collapsible_if,
    clippy::identity_op,
    clippy::len_without_is_empty,
    clippy::let_unit_value,
    clippy::new_without_default,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::too_many_arguments,
    clippy::type_complexity,
    non_camel_case_types,
    non_snake_case,
    unsafe_code
)]
#![deny(variant_size_differences)]
#![forbid(
    unused_results,
    unsafe_op_in_unsafe_fn,
    clippy::char_lit_as_u8,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_with_truncation,
    clippy::ptr_as_ptr
)]
#![warn(
    clippy::unnecessary_cast,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
#![cfg_attr(
    not(any(
        all(target_arch = "aarch64", target_endian = "little"),
        all(target_arch = "arm", target_endian = "little"),
        target_arch = "x86",
        target_arch = "x86_64",
        feature = "alloc"
    )),
    allow(dead_code, unused_imports, unused_macros)
)]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod debug;

#[macro_use]
mod prefixed;

#[doc(hidden)]
#[macro_use]
mod testutil;

#[macro_use]
mod bssl;

#[macro_use]
mod polyfill;

pub mod aead;

pub mod agreement;
mod arithmetic;
mod bits;

pub(crate) mod bb;
pub(crate) mod c;

#[doc(hidden)]
#[deprecated(
    note = "Will be removed. Internal module not intended for external use, with no promises regarding side channels."
)]
pub mod deprecated_constant_time;

#[doc(hidden)]
#[allow(deprecated)]
#[deprecated(
    note = "Will be removed. Internal module not intended for external use, with no promises regarding side channels."
)]
pub use deprecated_constant_time as constant_time;

pub mod io;

mod cpu;
pub mod digest;
mod ec;
pub mod error;
pub mod hkdf;
pub mod hmac;
mod limb;
pub mod pbkdf2;
pub mod pkcs8;
pub mod rand;

#[cfg(feature = "alloc")]
pub mod rsa;

pub mod signature;

#[cfg(test)]
mod tests;

mod sealed {
    /// Traits that are designed to only be implemented internally in *ring*.
    //
    // Usage:
    // ```
    // use crate::sealed;
    //
    // pub trait MyType: sealed::Sealed {
    //     // [...]
    // }
    //
    // impl sealed::Sealed for MyType {}
    // ```
    pub trait Sealed {}
}

#[deprecated(note = "internal API that will be removed")]
pub mod deprecated_test;

#[allow(deprecated)]
#[deprecated(note = "internal API that will be removed")]
pub use deprecated_test as test;
