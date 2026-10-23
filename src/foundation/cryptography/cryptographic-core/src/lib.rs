//! Shared cryptographic interfaces and optional block buffering.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![doc = include_str!("../../../../../docs/components/cryptographic-core.md")]

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

/// Block buffering, with its existing narrowly isolated unsafe implementation.
#[cfg(feature = "block-api")]
#[allow(unsafe_code)]
pub mod buffer;
/// Common cryptographic sizes, keys and initialization traits.
#[forbid(unsafe_code)]
pub mod common_types;
#[forbid(unsafe_code)]
mod hash;
pub use hash::*;
