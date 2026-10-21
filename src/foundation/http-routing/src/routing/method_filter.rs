use http::Method;
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

/// A filter that matches one or more HTTP methods.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MethodFilter(u16);

impl MethodFilter {
    /// Match `CONNECT` requests.
    ///
    /// This filter only selects the method; it does not implement a tunnel,
    /// an HTTP/2 extended CONNECT protocol, or WebSocket support.
    pub const CONNECT: Self = Self::from_bits(0b0_0000_0001);
    /// Match `DELETE` requests.
    pub const DELETE: Self = Self::from_bits(0b0_0000_0010);
    /// Match `GET` requests.
    pub const GET: Self = Self::from_bits(0b0_0000_0100);
    /// Match `HEAD` requests.
    pub const HEAD: Self = Self::from_bits(0b0_0000_1000);
    /// Match `OPTIONS` requests.
    pub const OPTIONS: Self = Self::from_bits(0b0_0001_0000);
    /// Match `PATCH` requests.
    pub const PATCH: Self = Self::from_bits(0b0_0010_0000);
    /// Match `POST` requests.
    pub const POST: Self = Self::from_bits(0b0_0100_0000);
    /// Match `PUT` requests.
    pub const PUT: Self = Self::from_bits(0b0_1000_0000);
    /// Match `TRACE` requests.
    pub const TRACE: Self = Self::from_bits(0b1_0000_0000);

    const fn bits(&self) -> u16 {
        let bits = self;
        bits.0
    }

    const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub(crate) const fn contains(&self, other: Self) -> bool {
        self.bits() & other.bits() == other.bits()
    }

    /// Performs the OR operation between the [`MethodFilter`] in `self` with `other`.
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Error type used when converting a [`Method`] to a [`MethodFilter`] fails.
#[derive(Debug)]
pub struct NoMatchingMethodFilter {
    method: Method,
}

impl NoMatchingMethodFilter {
    /// Get the [`Method`] that couldn't be converted to a [`MethodFilter`].
    pub fn method(&self) -> &Method {
        &self.method
    }
}

impl fmt::Display for NoMatchingMethodFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "no `MethodFilter` for `{}`", self.method.as_str())
    }
}

impl std::error::Error for NoMatchingMethodFilter {}

impl TryFrom<Method> for MethodFilter {
    type Error = NoMatchingMethodFilter;

    fn try_from(m: Method) -> Result<Self, NoMatchingMethodFilter> {
        match m {
            Method::CONNECT => Ok(MethodFilter::CONNECT),
            Method::DELETE => Ok(MethodFilter::DELETE),
            Method::GET => Ok(MethodFilter::GET),
            Method::HEAD => Ok(MethodFilter::HEAD),
            Method::OPTIONS => Ok(MethodFilter::OPTIONS),
            Method::PATCH => Ok(MethodFilter::PATCH),
            Method::POST => Ok(MethodFilter::POST),
            Method::PUT => Ok(MethodFilter::PUT),
            Method::TRACE => Ok(MethodFilter::TRACE),
            other => Err(NoMatchingMethodFilter { method: other }),
        }
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/http-routing/unit/src/routing/method_filter/tests_107.rs"
));
