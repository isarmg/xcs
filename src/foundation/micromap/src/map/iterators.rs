// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;
use core::{fmt, iter::FusedIterator, mem::MaybeUninit};

impl<K, V, const N: usize> Map<K, V, N> {
    /// An iterator visiting all key-value pairs in _non-deterministic order_.
    /// The iterator element type is `(&'a K, &'a V)`.
    ///
    /// If no mutable borrow modifications are made to the map before the next
    /// iteration, the order of iteration will not change.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let map = Map::from([("a", 1), ("b", 2), ("c", 3)]);
    /// for (k, v) in &map { // Implicit call as `map.iter()`
    ///     println!("key: {k} val: {v}");
    /// }
    /// ```
    ///
    /// # Performance
    /// In the current implementation, iterating over map takes `O(len)` time.
    /// The average complexity is `O(len/2)`.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            iter: self.pairs[..self.len].as_ref().iter(),
        }
    }

    /// An iterator visiting all key-value pairs in _arbitrary order_, with
    /// mutable references to the values. The iterator element type
    /// is `(&'a K, &'a mut V)`.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut map = Map::from([("a", 1), ("b", 2), ("c", 3)]);
    /// // Update all values
    /// for (_, val) in &mut map { // Implicit call as `map.iter_mut()`
    ///     *val *= 2;
    /// }
    /// for (k, v) in &map { // Implicit call as `map.iter()`
    ///     println!("key: {k} val: {v}"); // vals are now 2, 4, 6
    /// }
    /// ```
    ///
    /// # Performance
    /// In the current implementation, iterating over map takes `O(len)` time.
    /// The average complexity is `O(len/2)`.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            iter: self.pairs[..self.len].as_mut().iter_mut(),
        }
    }
}

/// An iterator over the entries of a `HashMap`.
///
/// This `struct` is created by the [`iter`][`Map::iter`] method on [`Map`].
/// See its documentation for more.
///
/// # Example
/// ```
/// use micromap::Map;
/// let map = Map::from([("a", 1), ("b", 2)]);
/// let mut iter = map.iter();
/// assert_eq!(iter.len(), 2);
/// assert_eq!(iter.next(), Some((&"a", &1)));
/// assert_eq!(iter.len(), 1);
/// assert_eq!(iter.next(), Some((&"b", &2)));
/// assert_eq!(iter.len(), 0);
/// assert_eq!(iter.next(), None);
/// ```
#[repr(transparent)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, K, V> {
    iter: core::slice::Iter<'a, MaybeUninit<(K, V)>>,
}

/// A mutable iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`iter_mut`][`Map::iter_mut`] method
/// on [`Map`]. See its documentation for more.
///
/// # Example
/// ```
/// use micromap::Map;
/// let mut map = Map::from([("a", 1), ("b", 2)]);
/// let mut iter = map.iter_mut();
/// iter.next().map(|(_, v)| *v *= 2); // ("a", 2)
/// iter.next().map(|(_, v)| *v += 1); // ("b", 3)
/// assert_eq!(iter.len(), 0);
/// assert_eq!(iter.next(), None);
/// assert_eq!(map, Map::from([("a", 2), ("b", 3)]));
/// ```
#[repr(transparent)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IterMut<'a, K, V> {
    pub(super) iter: core::slice::IterMut<'a, MaybeUninit<(K, V)>>,
}

/// An owning iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`into_iter`][`IntoIterator::into_iter`]
/// method on [`Map`] (provided by the [`IntoIterator`] trait). See its
/// documentation for more.
///
/// # Example
/// ```
/// use micromap::Map;
/// let map = Map::from([("a", 1)]);
/// let _iter = map.into_iter(); // consumed map
/// let map = Map::from([('b', 2), ('c', 3)]);
/// for (k, v) in map { // Implicit call as `map.into_iter()`
///     println!("key: {k} val: {v}");
/// }
/// // assert_eq!(map.len(), 2); // `into_iter()` takes ownership, so can not do this
/// ```
#[repr(transparent)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IntoIter<K, V, const N: usize> {
    pub(super) map: Map<K, V, N>,
}

/// Utility function for implementing Debug trait for iterators (whose inner is a slice).
#[inline]
pub fn slice_iter<K, V>(slice: &[MaybeUninit<(K, V)>]) -> impl Iterator<Item = (&K, &V)> {
    slice.iter().map(|may| unsafe {
        let (k, v) = may.assume_init_ref();
        (k, v)
    })
}

impl<K, V> Clone for Iter<'_, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(slice_iter(self.iter.as_slice()))
            .finish()
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IterMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(slice_iter(self.iter.as_slice()))
            .finish()
    }
}

impl<K: fmt::Debug, V: fmt::Debug, const N: usize> fmt::Debug for IntoIter<K, V, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.map.iter()).finish()
    }
}

impl<K, V> Default for Iter<'_, K, V> {
    #[inline]
    fn default() -> Self {
        Self { iter: [].iter() }
    }
}

impl<K, V> Default for IterMut<'_, K, V> {
    #[inline]
    fn default() -> Self {
        Self {
            iter: [].iter_mut(),
        }
    }
}

impl<K, V, const N: usize> Default for IntoIter<K, V, N> {
    #[inline]
    fn default() -> Self {
        Self {
            map: Map::default(),
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|p| {
            let p = unsafe { p.assume_init_ref() };
            (&p.0, &p.1)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|p| {
            let p = unsafe { p.assume_init_mut() };
            (&p.0, &mut p.1)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<K, V, const N: usize> Iterator for IntoIter<K, V, N> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.map.len > 0 {
            self.map.len -= 1;
            Some(unsafe { self.map.item_read(self.map.len) })
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.map.len, Some(self.map.len))
    }

    #[inline]
    fn count(self) -> usize {
        self.map.len()
    }
}

impl<'a, K, V, const N: usize> IntoIterator for &'a Map<K, V, N> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, const N: usize> IntoIterator for &'a mut Map<K, V, N> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V, const N: usize> IntoIterator for Map<K, V, N> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V, N>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { map: self }
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<K, V, const N: usize> ExactSizeIterator for IntoIter<K, V, N> {
    #[inline]
    fn len(&self) -> usize {
        self.map.len
    }
}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<K, V> FusedIterator for IterMut<'_, K, V> {}

impl<K, V, const N: usize> FusedIterator for IntoIter<K, V, N> {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/iterators/tests_319.rs"));
