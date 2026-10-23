// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Big Integer Types for Rust
//!
//! * A [`BigUint`] is unsigned and represented as a vector of digits.
//! * A [`BigInt`] is signed and is a combination of [`BigUint`] and [`Sign`].
//!
//! Common numerical operations are overloaded, so we can treat them
//! the same way we treat other numbers.
//!
//! ## Example
//!
//! ```rust
//! # fn main() {
//! use numeric::bigint::BigUint;
//! use numeric::traits::One;
//!
//! // Calculate large fibonacci numbers.
//! fn fib(n: usize) -> BigUint {
//!     let mut f0 = BigUint::ZERO;
//!     let mut f1 = BigUint::ONE;
//!     for _ in 0..n {
//!         let f2 = f0 + &f1;
//!         f0 = f1;
//!         f1 = f2;
//!     }
//!     f0
//! }
//!
//! // This is a very large number.
//! println!("fib(1000) = {}", fib(1000));
//! # }
//! ```
//!
//! ## Features
//!
//! Enable `numeric/bigint` for these allocation-backed types. The `std` feature
//! enables standard error implementations and floating-point approximations;
//! `default-features = false` keeps the allocation-only implementation.
//! Random generation and arbitrary-value provider integrations are not included
//! in this maintained component. See the numeric architecture document for the
//! removed unsupported feature inventory.
//!
//! ### Serialization
//!
//! The `serialization` feature adds implementations of [`Serialize`][data_serialization::Serialize] and
//! [`Deserialize`][data_serialization::Deserialize] for both [`BigInt`] and [`BigUint`]. Their serialized data is
//! generated portably, regardless of platform differences like the internal digit size.
//!
//!
//! ## Compatibility
//!
//! The `num-bigint` crate is tested for rustc 1.60 and greater.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]



use core::fmt;

#[macro_use]
mod macros;

mod bigint;
mod biguint;

#[cfg(target_pointer_width = "32")]
type UsizePromotion = u32;
#[cfg(target_pointer_width = "64")]
type UsizePromotion = u64;

#[cfg(target_pointer_width = "32")]
type IsizePromotion = i32;
#[cfg(target_pointer_width = "64")]
type IsizePromotion = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBigIntError {
    kind: BigIntErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BigIntErrorKind {
    Empty,
    InvalidDigit,
}

impl ParseBigIntError {
    fn __description(&self) -> &str {
        use crate::bigint::BigIntErrorKind::*;
        match self.kind {
            Empty => "cannot parse integer from empty string",
            InvalidDigit => "invalid digit found in string",
        }
    }

    fn empty() -> Self {
        ParseBigIntError {
            kind: BigIntErrorKind::Empty,
        }
    }

    fn invalid() -> Self {
        ParseBigIntError {
            kind: BigIntErrorKind::InvalidDigit,
        }
    }
}

impl fmt::Display for ParseBigIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.__description().fmt(f)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl std::error::Error for ParseBigIntError {
    fn description(&self) -> &str {
        self.__description()
    }
}

/// The error type returned when a checked conversion regarding big integer fails.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromBigIntError<T> {
    original: T,
}

impl<T> TryFromBigIntError<T> {
    fn new(original: T) -> Self {
        TryFromBigIntError { original }
    }

    fn __description(&self) -> &str {
        "out of range conversion regarding big integer attempted"
    }

    /// Extract the original value, if available. The value will be available
    /// if the type before conversion was either [`BigInt`] or [`BigUint`].
    pub fn into_original(self) -> T {
        self.original
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T> std::error::Error for TryFromBigIntError<T>
where
    T: fmt::Debug,
{
    fn description(&self) -> &str {
        self.__description()
    }
}

impl<T> fmt::Display for TryFromBigIntError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.__description().fmt(f)
    }
}

pub use crate::bigint::biguint::BigUint;
pub use crate::bigint::biguint::ToBigUint;
pub use crate::bigint::biguint::U32Digits;
pub use crate::bigint::biguint::U64Digits;

pub use crate::bigint::bigint::BigInt;
pub use crate::bigint::bigint::Sign;
pub use crate::bigint::bigint::ToBigInt;


mod big_digit;
