// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::vecs::{Index16, Index32};
use crate::{ule::VarULE, VarZeroSlice, VarZeroVec};
use databake::*;

impl<T: VarULE + ?Sized> Bake for VarZeroVec<'_, T, Index16> {
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        if self.is_empty() {
            quote! { zerovec::vecs::VarZeroVec16::new() }
        } else {
            let bytes = Bake::bake(&self.as_bytes(), env);
            // Safety: bytes was obtained from a VarZeroVec via as_bytes() above,
            // and thus is valid for unchecked construction.
            quote! { unsafe { zerovec::vecs::VarZeroVec16::from_bytes_unchecked(#bytes) } }
        }
    }
}

impl<T: VarULE + ?Sized> Bake for VarZeroVec<'_, T, Index32> {
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        if self.is_empty() {
            quote! { zerovec::vecs::VarZeroVec32::new() }
        } else {
            let bytes = Bake::bake(&self.as_bytes(), env);
            // Safety: bytes was obtained from a VarZeroVec via as_bytes() above,
            // and thus is valid for unchecked construction.
            quote! { unsafe { zerovec::vecs::VarZeroVec32::from_bytes_unchecked(#bytes) } }
        }
    }
}

impl<T: VarULE + ?Sized> BakeSize for VarZeroVec<'_, T, Index16> {
    fn borrows_size(&self) -> usize {
        self.as_bytes().len()
    }
}

impl<T: VarULE + ?Sized> BakeSize for VarZeroVec<'_, T, Index32> {
    fn borrows_size(&self) -> usize {
        self.as_bytes().len()
    }
}

impl<T: VarULE + ?Sized> Bake for &VarZeroSlice<T, Index16> {
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        if self.is_empty() {
            quote! { zerovec::vecs::VarZeroSlice16::new_empty() }
        } else {
            let bytes = Bake::bake(&self.as_bytes(), env);
            // Safety: bytes was obtained from a VarZeroSlice via as_bytes() above,
            // and thus is valid for unchecked construction.
            quote! { unsafe { zerovec::vecs::VarZeroSlice16::from_bytes_unchecked(#bytes) } }
        }
    }
}

impl<T: VarULE + ?Sized> Bake for &VarZeroSlice<T, Index32> {
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        if self.is_empty() {
            quote! { zerovec::vecs::VarZeroSlice32::new_empty() }
        } else {
            let bytes = Bake::bake(&self.as_bytes(), env);
            // Safety: bytes was obtained from a VarZeroSlice via as_bytes() above,
            // and thus is valid for unchecked construction.
            quote! { unsafe { zerovec::vecs::VarZeroSlice32::from_bytes_unchecked(#bytes) } }
        }
    }
}

impl<T: VarULE + ?Sized> BakeSize for &VarZeroSlice<T, Index16> {
    fn borrows_size(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            self.as_bytes().len()
        }
    }
}

impl<T: VarULE + ?Sized> BakeSize for &VarZeroSlice<T, Index32> {
    fn borrows_size(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            self.as_bytes().len()
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/zerovec/functions/src/varzerovec/databake/test_baked_vec_97.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/zerovec/functions/src/varzerovec/databake/test_baked_slice_116.rs"));
