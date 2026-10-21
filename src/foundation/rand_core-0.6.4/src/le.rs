// Copyright 2018 Developers of the Rand project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Little-Endian utilities
//!
//! Little-Endian order has been chosen for internal usage; this makes some
//! useful functions available.

use core::convert::TryInto;

/// Reads unsigned 32 bit integers from `src` into `dst`.
#[inline]
pub fn read_u32_into(src: &[u8], dst: &mut [u32]) {
    assert!(src.len() >= 4 * dst.len());
    for (out, chunk) in dst.iter_mut().zip(src.chunks_exact(4)) {
        *out = u32::from_le_bytes(chunk.try_into().unwrap());
    }
}

/// Reads unsigned 64 bit integers from `src` into `dst`.
#[inline]
pub fn read_u64_into(src: &[u8], dst: &mut [u64]) {
    assert!(src.len() >= 8 * dst.len());
    for (out, chunk) in dst.iter_mut().zip(src.chunks_exact(8)) {
        *out = u64::from_le_bytes(chunk.try_into().unwrap());
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rand_core-0.6.4/functions/src/le/test_read_34.rs"));
