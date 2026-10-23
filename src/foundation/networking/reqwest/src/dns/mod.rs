//! DNS resolution

pub use resolve::{Addrs, Name, Resolve, Resolving};
pub(crate) use resolve::{DnsResolverWithOverrides, DynResolver};

#[cfg(docsrs)]
pub use resolve::IntoResolve;

pub(crate) mod gai;

pub(crate) mod resolve;
