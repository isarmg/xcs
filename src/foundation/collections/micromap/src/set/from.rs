// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Set;

impl<T: PartialEq, const N: usize> FromIterator<T> for Set<T, N> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut s: Self = Self::new();
        for k in iter {
            s.insert(k);
        }
        s
    }
}

impl<T: PartialEq, const N: usize> From<[T; N]> for Set<T, N> {
    #[inline]
    fn from(arr: [T; N]) -> Self {
        Self::from_iter(arr)
    }
}
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/micromap/unit/src/set/from/tests_23.rs"));
