// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::difference::Difference;
use super::iterators::Iter;
use super::Set;

impl<T: PartialEq, const N: usize> Set<T, N> {
    /// Visits the values representing the union,
    /// i.e., the values that are in `self` or `other`.
    ///
    /// # Examples
    /// ```
    /// use micromap::Set;
    /// let a = Set::from([1, 2, 3]);
    /// let b = Set::from([4, 2, 3, 4]);
    /// // Can be seen as `a ∪ b`.
    /// for x in a.union(&b) {
    ///     println!("{x}"); // Print 1, 2, 3, 4
    /// }
    /// let union: Set<_, 7> = a.union(&b).copied().collect();
    /// assert_eq!(union, Set::from([1, 2, 3, 4]));
    /// ```
    #[inline]
    pub fn union<'a, const M: usize>(&'a self, other: &'a Set<T, M>) -> Union<'a, T, M> {
        Union {
            iter: other.iter().chain(self.difference(other)),
        }
    }
}

/// A lazy iterator producing elements in the union of Linear `Set`s.
///
/// This `struct` is created by the [`union`] method on [`Set`].
///
/// [`union`]: Set::union
///
/// # Examples
/// ```
/// use micromap::Set;
/// let a = Set::from([1, 2, 3]);
/// let b = Set::from([4, 2, 3, 4]);
/// let union = a.union(&b);
/// assert_eq!(union.count(), 4);
/// ```
#[must_use = "this returns the union as an iterator, without modifying either \
              input set"]
pub struct Union<'a, T, const M: usize> {
    iter: core::iter::Chain<Iter<'a, T>, Difference<'a, 'a, T, M>>,
}

impl<T: PartialEq, const M: usize> Clone for Union<'_, T, M> {
    #[inline]
    fn clone(&self) -> Self {
        Union {
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T: PartialEq, const M: usize> Iterator for Union<'a, T, M> {
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
    fn count(self) -> usize {
        self.iter.count()
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

impl<T: core::fmt::Debug + PartialEq, const M: usize> core::fmt::Debug for Union<'_, T, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<T: PartialEq, const M: usize> core::iter::FusedIterator for Union<'_, T, M> {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/union/tests_98.rs"));
