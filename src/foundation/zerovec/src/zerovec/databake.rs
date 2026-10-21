// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use super::ZeroVec;
use crate::{ule::AsULE, ZeroSlice};
use databake::*;

impl<T: AsULE> Bake for ZeroVec<'_, T> {
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        if self.is_empty() {
            quote! { zerovec::ZeroVec::new() }
        } else {
            let bytes = Bake::bake(&self.as_bytes(), env);
            // Safety: bytes was obtained from a ZeroVec via as_bytes() above,
            // and thus is valid for unchecked construction.
            quote! { unsafe { zerovec::ZeroVec::from_bytes_unchecked(#bytes) } }
        }
    }
}

impl<T: AsULE> BakeSize for ZeroVec<'_, T> {
    fn borrows_size(&self) -> usize {
        self.as_bytes().len()
    }
}

impl<T: AsULE> Bake for &ZeroSlice<T> {
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        if self.is_empty() {
            quote! { zerovec::ZeroSlice::new_empty() }
        } else {
            let bytes = Bake::bake(&self.as_bytes(), env);
            // Safety: bytes was obtained from a ZeroSlice via as_bytes() above,
            // and thus is valid for unchecked construction.
            quote! { unsafe { zerovec::ZeroSlice::from_bytes_unchecked(#bytes) } }
        }
    }
}

impl<T: AsULE> BakeSize for &ZeroSlice<T> {
    fn borrows_size(&self) -> usize {
        self.as_bytes().len()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerovec/functions/src/zerovec/databake/test_baked_vec_49.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerovec/functions/src/zerovec/databake/test_baked_slice_60.rs"));
