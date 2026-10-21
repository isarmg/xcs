// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::Set;

impl<T: PartialEq, const N: usize> Extend<T> for Set<T, N> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|item| {
            self.insert(item);
        });
    }
}

impl<'a, T: 'a + PartialEq + Copy, const N: usize> Extend<&'a T> for Set<T, N> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().copied());
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/extend/tests_23.rs"));
