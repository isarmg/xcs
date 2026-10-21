// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;

impl<K: PartialEq, V, const N: usize> FromIterator<(K, V)> for Map<K, V, N> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut m: Self = Self::new();
        for (k, v) in iter {
            m.insert(k, v);
        }
        m
    }
}

impl<K: PartialEq, V, const N: usize> From<[(K, V); N]> for Map<K, V, N> {
    #[inline]
    fn from(arr: [(K, V); N]) -> Self {
        Self::from_iter(arr)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/from/tests_24.rs"));
