// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::{maps::ZeroMapBorrowed, maps::ZeroMapKV, ZeroMap};
use databake::*;

impl<'a, K, V> Bake for ZeroMap<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    K::Container: Bake,
    V::Container: Bake,
{
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        let keys = self.keys.bake(env);
        let values = self.values.bake(env);
        quote! { unsafe { #[allow(unused_unsafe)] zerovec::ZeroMap::from_parts_unchecked(#keys, #values) } }
    }
}

impl<'a, K, V> BakeSize for ZeroMap<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    K::Container: BakeSize,
    V::Container: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.keys.borrows_size() + self.values.borrows_size()
    }
}

impl<'a, K, V> Bake for ZeroMapBorrowed<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    &'a K::Slice: Bake,
    &'a V::Slice: Bake,
{
    fn bake(&self, env: &CrateEnv) -> TokenStream {
        env.insert("zerovec");
        let keys = self.keys.bake(env);
        let values = self.values.bake(env);
        quote! { unsafe { #[allow(unused_unsafe)] zerovec::maps::ZeroMapBorrowed::from_parts_unchecked(#keys, #values) } }
    }
}

impl<'a, K, V> BakeSize for ZeroMapBorrowed<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    &'a K::Slice: BakeSize,
    &'a V::Slice: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.keys.borrows_size() + self.values.borrows_size()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerovec/functions/src/map/databake/test_baked_map_62.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerovec/functions/src/map/databake/test_baked_borrowed_map_86.rs"));
