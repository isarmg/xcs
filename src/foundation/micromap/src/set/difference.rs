// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::iterators::Iter;
use super::Set;

impl<T: PartialEq, const N: usize> Set<T, N> {
    /// Visits the values representing the difference,
    /// i.e., the values that are in `self` but not in `other`.
    ///
    /// # Examples
    /// ```
    /// use micromap::Set;
    /// let a = Set::from([1, 2, 3]);
    /// let b = Set::from([4, 2, 3, 4]);
    /// // Can be seen as `a - b`.
    /// for x in a.difference(&b) {
    ///     println!("{x}"); // Print 1
    /// }
    /// let diff: Set<_, 3> = a.difference(&b).copied().collect();
    /// assert_eq!(diff, Set::from([1]));
    /// // Note that difference is not symmetric,
    /// // and `b - a` means something else:
    /// let diff: Set<_, 4> = b.difference(&a).copied().collect();
    /// assert_eq!(diff, Set::from([4]));
    /// ```
    #[inline]
    pub fn difference<'a, 'b, const M: usize>(
        &'a self,
        other: &'b Set<T, M>,
    ) -> Difference<'a, 'b, T, M> {
        Difference {
            iter: self.iter(),
            other,
        }
    }
}

/// A lazy iterator producing elements in the difference of Linear `Set`s.
///
/// This `struct` is created by the [`difference`] method on [`Set`].
///
/// [`difference`]: Set::difference
///
/// # Examples
/// ```
/// use micromap::Set;
/// let a = Set::from([1, 2, 3]);
/// let b = Set::from([4, 2, 3, 4]);
/// let diff = a.difference(&b);
/// assert_eq!(diff.count(), 1);
/// ```
#[must_use = "this returns the difference as an iterator, without modifying \
              either input set"]
pub struct Difference<'a, 'b, T, const M: usize> {
    // iterator of the first set
    iter: Iter<'a, T>,
    // the second set
    other: &'b Set<T, M>,
}

impl<T, const M: usize> Clone for Difference<'_, '_, T, M> {
    #[inline]
    fn clone(&self) -> Self {
        Difference {
            iter: self.iter.clone(),
            ..*self
        }
    }
}

impl<'a, T: PartialEq, const M: usize> Iterator for Difference<'a, '_, T, M> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|&item| !self.other.contains(item))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let lower = if self.iter.len() > self.other.len() {
            self.iter.len() - self.other.len()
        } else {
            0
        };
        let (_, upper) = self.iter.size_hint();
        (lower, upper)
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
                acc
            } else {
                f(acc, elt)
            }
        })
    }
}

impl<T: PartialEq, const M: usize> core::iter::FusedIterator for Difference<'_, '_, T, M> {}

impl<T: core::fmt::Debug + PartialEq, const M: usize> core::fmt::Debug
    for Difference<'_, '_, T, M>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub mod difference_ref {
    use super::{Iter, Set};

    impl<'a, T: PartialEq + ?Sized, const N: usize> Set<&'a T, N> {
        /// A [`difference()`][Set::difference] method with more elaborate lifetime and
        /// just for `Set<&T>`.
        #[inline]
        pub fn difference_ref<'b, 'set1, 'set2, const M: usize>(
            &'set1 self,
            other: &'set2 Set<&'b T, M>,
        ) -> DifferenceRef<'a, 'b, 'set2, T, M>
        where
            'set1: 'a,
            'set2: 'b,
        {
            DifferenceRef {
                iter: self.iter(),
                other,
            }
        }
    }

    /// A lazy iterator producing reference elements in the difference of
    /// Linear `Set`s.
    ///
    /// This `struct` is created by the [`difference_ref`] method on [`Set`].
    ///
    /// [`difference_ref`]: Set::difference_ref
    ///
    /// # Examples
    /// ```
    /// use micromap::Set;
    /// let a = Set::from(["foo", "bar", "baz"]);
    /// let b = Set::from(["fox", "foo", "baz", "bro"]);
    /// let diff = (&a).difference_ref(&b);
    /// assert_eq!(diff.count(), 1);
    /// ```
    #[must_use = "this returns the difference as an iterator, without modifying \
                  either input set"]
    pub struct DifferenceRef<'a, 'b, 'set, T: ?Sized, const M: usize> {
        // iterator of the first set
        iter: Iter<'a, &'a T>,
        // the second set
        other: &'set Set<&'b T, M>,
    }

    impl<T, const M: usize> Clone for DifferenceRef<'_, '_, '_, T, M> {
        #[inline]
        fn clone(&self) -> Self {
            DifferenceRef {
                iter: self.iter.clone(),
                ..*self
            }
        }
    }

    impl<'a, T: PartialEq + ?Sized, const M: usize> Iterator for DifferenceRef<'a, '_, '_, T, M> {
        type Item = &'a T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.find(|&item| !self.other.contains(item)).copied()
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let lower = if self.iter.len() > self.other.len() {
                self.iter.len() - self.other.len()
            } else {
                0
            };
            let (_, upper) = self.iter.size_hint();
            (lower, upper)
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
                    acc
                } else {
                    f(acc, elt)
                }
            })
        }
    }

    impl<T: PartialEq, const M: usize> core::iter::FusedIterator for DifferenceRef<'_, '_, '_, T, M> {}

    impl<T: core::fmt::Debug + PartialEq, const M: usize> core::fmt::Debug
        for DifferenceRef<'_, '_, '_, T, M>
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_list().entries(self.clone()).finish()
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/difference/tests_221.rs"));
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/difference/tests_308.rs"));
