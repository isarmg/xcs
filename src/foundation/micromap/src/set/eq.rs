// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::Set;

impl<T: PartialEq, const N: usize, const M: usize> PartialEq<Set<T, M>> for Set<T, N> {
    /// Two sets can be compared. (The capacity does not affect comparison.)
    ///
    /// # Examples
    /// ```
    /// let mut m1: micromap::Set<_, 5> = micromap::Set::new();
    /// let mut m2: micromap::Set<_, 10> = micromap::Set::new();
    /// m1.insert(1);
    /// m2.insert(1);
    /// assert_eq!(m1, m2);
    /// // two sets with different order of key-value pairs are still equal:
    /// m1.insert(2);
    /// m1.insert(3);
    /// m2.insert(3);
    /// m2.insert(2);
    /// assert_eq!(m1, m2);
    /// ```
    #[inline]
    fn eq(&self, other: &Set<T, M>) -> bool {
        self.map.eq(&other.map)
    }
}

impl<T: Eq, const N: usize> Eq for Set<T, N> {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/eq/tests_32.rs"));
