// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::Set;

// If we can use `#![feature(generic_const_exprs)]`, use `Set<T, min(N, M)>` as the
// const generic parameter to replace the `Set<T, N>`.
impl<T, const M: usize, const N: usize> core::ops::BitAnd<&Set<T, M>> for &Set<T, N>
where
    T: PartialEq + Clone,
{
    type Output = Set<T, N>;

    /// Returns the intersection of `self` and `rhs` as a new `Set<T, N>`.
    ///
    /// # Examples
    /// ```
    /// use micromap::Set;
    /// let a = Set::from([1, 2, 3]);
    /// let b = Set::from([2, 3, 4, 5]);
    /// let set = &a & &b;
    /// let mut i = 0;
    /// let expected = [2, 3];
    /// for x in &set {
    ///     assert!(expected.contains(x));
    ///     i += 1;
    /// }
    /// assert_eq!(i, expected.len());
    /// ```
    #[inline]
    fn bitand(self, rhs: &Set<T, M>) -> Set<T, N> {
        self.intersection(rhs).cloned().collect()
    }
}
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/micromap/unit/src/set/bitand/tests_36.rs"));
