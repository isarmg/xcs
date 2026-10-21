// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;
use core::borrow::Borrow;
use core::ops::{Index, IndexMut};

impl<K: PartialEq + Borrow<Q>, Q: PartialEq + ?Sized, V, const N: usize> Index<&Q>
    for Map<K, V, N>
{
    type Output = V;

    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("No entry found for the key")
    }
}

impl<K: PartialEq + Borrow<Q>, Q: PartialEq + ?Sized, V, const N: usize> IndexMut<&Q>
    for Map<K, V, N>
{
    #[inline]
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key).expect("No entry found for the key")
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/index/tests_28.rs"));
