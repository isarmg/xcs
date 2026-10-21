//! Implementations of commonly used traits like `Digest` and `Mac` from the
//! [`digest`](https://crates.io/crates/digest) crate.

pub use digest;

use crate::{Hasher, OutputReader};
use digest::array::{Array, typenum::U32, typenum::U64};
use digest::common;

impl digest::HashMarker for Hasher {}

impl digest::Update for Hasher {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.update(data);
    }
}

impl digest::Reset for Hasher {
    #[inline]
    fn reset(&mut self) {
        self.reset(); // the inherent method
    }
}

impl digest::OutputSizeUser for Hasher {
    type OutputSize = U32;
}

impl digest::FixedOutput for Hasher {
    #[inline]
    fn finalize_into(self, out: &mut Array<u8, Self::OutputSize>) {
        out.copy_from_slice(self.finalize().as_bytes());
    }
}

impl digest::FixedOutputReset for Hasher {
    #[inline]
    fn finalize_into_reset(&mut self, out: &mut Array<u8, Self::OutputSize>) {
        out.copy_from_slice(self.finalize().as_bytes());
        self.reset();
    }
}

impl digest::ExtendableOutput for Hasher {
    type Reader = OutputReader;

    #[inline]
    fn finalize_xof(self) -> Self::Reader {
        Hasher::finalize_xof(&self)
    }
}

impl digest::ExtendableOutputReset for Hasher {
    #[inline]
    fn finalize_xof_reset(&mut self) -> Self::Reader {
        let reader = Hasher::finalize_xof(self);
        self.reset();
        reader
    }
}

impl digest::XofReader for OutputReader {
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) {
        self.fill(buffer);
    }
}

impl common::KeySizeUser for Hasher {
    type KeySize = U32;
}

impl common::BlockSizeUser for Hasher {
    type BlockSize = U64;
}

impl digest::MacMarker for Hasher {}

impl digest::KeyInit for Hasher {
    #[inline]
    fn new(key: &digest::Key<Self>) -> Self {
        let key_bytes: [u8; 32] = (*key).into();
        Hasher::new_keyed(&key_bytes)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/blake3/unit/src/traits/test_88.rs"));
