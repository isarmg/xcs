// Copyright 2018 Developers of the Rand project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Error types

use core::fmt;
use core::num::NonZeroU32;

#[cfg(feature = "std")] use std::boxed::Box;

/// Error type of random number generators
///
/// In order to be compatible with `std` and `no_std`, this type has two
/// possible implementations: with `std` a boxed `Error` trait object is stored,
/// while with `no_std` we retain either a numeric user code or the complete
/// system entropy error. Entropy errors are never coerced into numeric user codes.
pub struct Error {
    #[cfg(feature = "std")]
    inner: Box<dyn std::error::Error + Send + Sync + 'static>,
    #[cfg(not(feature = "std"))]
    inner: ErrorKind,
}

#[cfg(not(feature = "std"))]
enum ErrorKind {
    Code(NonZeroU32),
    #[cfg(feature = "system-entropy")]
    Entropy(system_entropy::Error),
}

impl Error {
    /// Codes at or above this point can be used by users to define their own
    /// custom errors.
    ///
    /// This has a fixed value of `(1 << 31) + (1 << 30) = 0xC000_0000`,
    /// therefore the number of values available for custom codes is `1 << 30`.
    ///
    /// This namespace applies only to explicit numeric user errors.
    pub const CUSTOM_START: u32 = (1 << 31) + (1 << 30);
    /// Codes below this point represent OS Errors (i.e. positive i32 values).
    /// Codes at or above this point, but below [`Error::CUSTOM_START`] are
    /// reserved for use by the `rand` and `getrandom` crates.
    ///
    /// System entropy errors retain their original typed representation instead.
    pub const INTERNAL_START: u32 = 1 << 31;

    /// Construct from any type supporting `std::error::Error`
    ///
    /// Available only when configured with `std`.
    ///
    /// See also `From<NonZeroU32>`, which is available with and without `std`.
    #[cfg(feature = "std")]

    #[inline]
    pub fn new<E>(err: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        Error { inner: err.into() }
    }

    /// Reference the inner error (`std` only)
    ///
    /// When configured with `std`, this is a trivial operation and never
    /// panics. Without `std`, this method is simply unavailable.
    #[cfg(feature = "std")]

    #[inline]
    pub fn inner(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.inner
    }

    /// Unwrap the inner error (`std` only)
    ///
    /// When configured with `std`, this is a trivial operation and never
    /// panics. Without `std`, this method is simply unavailable.
    #[cfg(feature = "std")]

    #[inline]
    pub fn take_inner(self) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self.inner
    }

    /// Extract the raw OS error code (if this error came from the OS)
    ///
    /// This method is identical to `std::io::Error::raw_os_error()`, except
    /// that it works in `no_std` contexts. If this method returns `None`, the
    /// error value can still be formatted via the `Display` implementation.
    #[inline]
    pub fn raw_os_error(&self) -> Option<i32> {
        #[cfg(feature = "std")]
        {
            if let Some(e) = self.inner.downcast_ref::<std::io::Error>() {
                return e.raw_os_error();
            }
        }
        #[cfg(feature = "system-entropy")]
        {
            #[cfg(feature = "std")]
            let entropy = self.inner.downcast_ref::<system_entropy::Error>();
            #[cfg(not(feature = "std"))]
            let entropy = match &self.inner {
                ErrorKind::Entropy(error) => Some(error),
                ErrorKind::Code(_) => None,
            };
            if let Some(error) = entropy {
                return error.raw_os_error();
            }
        }
        match self.code() {
            Some(code) if u32::from(code) < Self::INTERNAL_START => Some(u32::from(code) as i32),
            _ => None,
        }
    }

    /// Retrieve the error code, if any.
    ///
    /// If this `Error` was constructed via `From<NonZeroU32>`, then this method
    /// will return this `NonZeroU32` code. Typed entropy errors return `None`
    /// in both `std` and `no_std`; inspect `raw_os_error` or format the error.
    #[inline]
    pub fn code(&self) -> Option<NonZeroU32> {
        #[cfg(feature = "std")]
        {
            self.inner.downcast_ref::<ErrorCode>().map(|c| c.0)
        }
        #[cfg(not(feature = "std"))]
        {
            match &self.inner {
                ErrorKind::Code(code) => Some(*code),
                #[cfg(feature = "system-entropy")]
                ErrorKind::Entropy(_) => None,
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            fmt::Debug::fmt(&self.inner, f)
        }
        #[cfg(not(feature = "std"))]
        {
            match &self.inner {
                ErrorKind::Code(code) => write!(f, "Error {{ code: {} }}", code),
                #[cfg(feature = "system-entropy")]
                ErrorKind::Entropy(error) => fmt::Debug::fmt(error, f),
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            fmt::Display::fmt(&self.inner, f)
        }
        #[cfg(not(feature = "std"))]
        {
            match &self.inner {
                ErrorKind::Code(code) => write!(f, "error code {}", code),
                #[cfg(feature = "system-entropy")]
                ErrorKind::Entropy(error) => fmt::Display::fmt(error, f),
            }
        }
    }
}

impl From<NonZeroU32> for Error {
    #[inline]
    fn from(code: NonZeroU32) -> Self {
        #[cfg(feature = "std")]
        {
            Error {
                inner: Box::new(ErrorCode(code)),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Error { inner: ErrorKind::Code(code) }
        }
    }
}

#[cfg(feature = "system-entropy")]
impl From<system_entropy::Error> for Error {
    #[inline]
    fn from(error: system_entropy::Error) -> Self {
        #[cfg(feature = "std")]
        {
            Error {
                inner: Box::new(error),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Error { inner: ErrorKind::Entropy(error) }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

#[cfg(feature = "std")]
impl From<Error> for std::io::Error {
    #[inline]
    fn from(error: Error) -> Self {
        if let Some(code) = error.raw_os_error() {
            std::io::Error::from_raw_os_error(code)
        } else {
            std::io::Error::new(std::io::ErrorKind::Other, error)
        }
    }
}

#[cfg(feature = "std")]
#[derive(Debug, Copy, Clone)]
struct ErrorCode(NonZeroU32);

#[cfg(feature = "std")]
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error code {}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ErrorCode {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/entropy/error.rs"));
