// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::iterators::Iter;
use super::Set;

impl<T: PartialEq, const N: usize> Set<T, N> {
    /// Visits the values representing the intersection,
    /// i.e., the values that are both in `self` and `other`.
    ///
    /// When an equal element is present in `self` and `other`,
    /// unlike the standard library functions, the resulting `Intersection`
    /// will ALWAYS yield references to the caller(`self`). This can be
    /// relevant if `T` contains fields which are not compared by its `Eq`
    /// implementation, and may hold different value between the two equal
    /// copies of `T` in the two sets.
    ///
    /// # Examples
    /// ```
    /// use micromap::Set;
    /// let a = Set::from([1, 2, 3]);
    /// let b = Set::from([4, 2, 3, 4]);
    /// // Print 2, 3 in arbitrary order.
    /// for x in a.intersection(&b) {
    ///     println!("{x}");
    /// }
    /// let intersection: Set<_, 3> = a.intersection(&b).copied().collect();
    /// assert_eq!(intersection, Set::from([2, 3]));
    /// ```
    #[inline]
    pub fn intersection<'a, const M: usize>(
        &'a self,
        other: &'a Set<T, M>,
    ) -> Intersection<'a, T, M> {
        Intersection {
            iter: self.iter(),
            other,
        }
    }
}

/// A lazy iterator producing elements in the intersection of Linear `Set`s.
///
/// This `struct` is created by the [`intersection`] method on [`Set`].
/// See its documentation for more.
///
/// [`intersection`]: Set::intersection
///
/// # Examples
/// ```
/// use micromap::Set;
/// let a = Set::from([1, 2, 3]);
/// let b = Set::from([4, 2, 3, 4]);
/// let intersection = a.intersection(&b);
/// assert_eq!(intersection.count(), 2);
/// ```
#[must_use = "this returns the intersection as an iterator, without modifying \
              either input set"]
pub struct Intersection<'a, T, const M: usize> {
    // iterator of the first set
    iter: Iter<'a, T>,
    // the second set
    other: &'a Set<T, M>,
}

impl<T, const M: usize> Clone for Intersection<'_, T, M> {
    #[inline]
    fn clone(&self) -> Self {
        Intersection {
            iter: self.iter.clone(),
            ..*self
        }
    }
}

impl<'a, T: PartialEq, const M: usize> Iterator for Intersection<'a, T, M> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.by_ref().find(|&item| self.other.contains(item))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(usize::min(self.iter.len(), self.other.len())))
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        // Maybe using iterator is better than the default Iterator::fold() which uses while loop.
        self.iter.fold(init, |acc, elt| {
            if self.other.contains(elt) {
                f(acc, elt)
            } else {
                acc
            }
        })
    }
}

impl<T: PartialEq, const M: usize> core::iter::FusedIterator for Intersection<'_, T, M> {}

impl<T: PartialEq + core::fmt::Debug, const M: usize> core::fmt::Debug for Intersection<'_, T, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/intersection/tests_115.rs"));
