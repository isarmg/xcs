// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::difference::Difference;
use super::Set;

impl<T: PartialEq, const N: usize> Set<T, N> {
    /// Visits the values representing the symmetric difference,
    /// i.e., the values that are in `self` or `other` but not in both.
    ///
    /// # Examples
    /// ```
    /// use micromap::Set;
    /// let a = Set::from([1, 2, 3]);
    /// let b = Set::from([4, 2, 3, 4]);
    /// // Can be seen as `(a - b) ∪ (b - a)`.
    /// for x in a.symmetric_difference(&b) {
    ///     println!("{x}"); // Print 1, 4
    /// }
    /// let sym_diff: Set<_, 7> = a.symmetric_difference(&b).copied().collect();
    /// assert_eq!(sym_diff, Set::from([1, 4]));
    /// ```
    #[inline]
    pub fn symmetric_difference<'a, const M: usize>(
        &'a self,
        other: &'a Set<T, M>,
    ) -> SymmetricDifference<'a, T, N, M> {
        SymmetricDifference {
            iter: self.difference(other).chain(other.difference(self)),
        }
    }
}

/// A lazy iterator producing elements in the symmetric difference of Linear `Set`s.
///
/// This `struct` is created by the [`symmetric_difference`] method on [`Set`].
///
/// [`symmetric_difference`]: Set::symmetric_difference
///
/// # Examples
/// ```
/// use micromap::Set;
/// let a = Set::from([1, 2, 3]);
/// let b = Set::from([4, 2, 3, 4]);
/// let sym_diff = a.symmetric_difference(&b);
/// assert_eq!(sym_diff.count(), 2);
/// ```
#[must_use = "this returns the difference as an iterator, without modifying \
              either input set"]
pub struct SymmetricDifference<'a, T, const N: usize, const M: usize> {
    iter: core::iter::Chain<Difference<'a, 'a, T, M>, Difference<'a, 'a, T, N>>,
}

impl<T, const N: usize, const M: usize> Clone for SymmetricDifference<'_, T, N, M> {
    #[inline]
    fn clone(&self) -> Self {
        SymmetricDifference {
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T: PartialEq, const N: usize, const M: usize> Iterator
    for SymmetricDifference<'a, T, N, M>
{
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter.fold(init, f)
    }
}

impl<T: PartialEq + core::fmt::Debug, const N: usize, const M: usize> core::fmt::Debug
    for SymmetricDifference<'_, T, N, M>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<T: PartialEq, const N: usize, const M: usize> core::iter::FusedIterator
    for SymmetricDifference<'_, T, N, M>
{
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/micromap/unit/src/set/symmetric_difference/tests_102.rs"));
