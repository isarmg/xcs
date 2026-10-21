// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::Set;
use core::ops::Sub;

impl<T, const N: usize, const M: usize> Sub<&Set<T, M>> for &Set<T, N>
where
    T: PartialEq + Clone,
{
    type Output = Set<T, N>;

    /// Returns the difference of `self` and `rhs` as a new `Set<T, N>`.
    /// The capacity of return set is same as `Self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use micromap::Set;
    ///
    /// let a = Set::from([0, 1, 2, 3, 4]);
    /// let b = Set::from([1, 3, 4, 5]);
    /// let set = &a - &b;
    /// let expected = Set::from([0, 2]);
    ///
    /// assert_eq!(set, expected);
    /// ```
    #[inline]
    fn sub(self, rhs: &Set<T, M>) -> Set<T, N> {
        self.difference(rhs).cloned().collect()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/sub/tests_35.rs"));
