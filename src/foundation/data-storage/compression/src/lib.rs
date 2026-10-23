//! DEFLATE codecs, stream formats, and Adler-32 checksums.
//! Checksum-only consumers need no allocator, standard library, or dependencies.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
#[cfg_attr(feature = "streams", macro_use)]
extern crate alloc;

pub mod checksum;
#[cfg(feature = "simd")]
pub mod accelerated_checksum;
#[cfg(feature = "codec")]
pub mod codec;
#[cfg(feature = "streams")]
pub mod streams;
