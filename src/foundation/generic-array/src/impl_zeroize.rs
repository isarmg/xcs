use crate::{ArrayLength, GenericArray};

use zeroize::Zeroize;

impl<T: Zeroize, N: ArrayLength<T>> Zeroize for GenericArray<T, N> {
    fn zeroize(&mut self) {
        self.as_mut_slice().iter_mut().zeroize()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/generic-array/unit/src/impl_zeroize/tests_11.rs"));
