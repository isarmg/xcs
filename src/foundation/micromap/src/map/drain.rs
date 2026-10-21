// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::iterators::slice_iter;
use super::Map;
use core::{fmt, iter::FusedIterator, mem::MaybeUninit};

impl<K, V, const N: usize> Map<K, V, N> {
    /// Clears the map, returning all key-value pairs as an iterator. For reuse, the
    /// memory of capacity `N` will be keeped.
    ///
    /// If the returned iterator is dropped before being fully consumed, it drops the
    /// remaining key-value pairs. The returned iterator keeps a mutable borrow on
    /// the map to optimize its implementation.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut a = Map::<_, _, 3>::new();
    /// a.insert(1, "a");
    /// a.insert(2, "b");
    /// for (k, v) in a.drain().take(1) {
    ///     assert!(k == 1 || k == 2);
    ///     assert!(v == "a" || v == "b");
    /// }
    /// assert!(a.is_empty());
    /// ```
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        let drain = Drain {
            iter: self.pairs[0..self.len].iter_mut(),
        };
        self.len = 0;
        drain
    }
}

/// A draining iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`drain`][`Map::drain`] method on [`Map`].
/// See its documentation for more.
///
/// # Example
/// ```
/// use micromap::Map;
/// let mut map = Map::from([("a", 1), ("b", 2)]);
/// assert_eq!(map.len(), 2);
/// let iter = map.drain();
/// assert_eq!(iter.len(), 2);
/// // assert_eq!(map.len(), 0); // Drain keeps a mutable borrow on the map
/// drop(iter);
/// assert_eq!(map.len(), 0); // Now we can use the map's borrow
/// ```
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Drain<'a, K, V> {
    iter: core::slice::IterMut<'a, MaybeUninit<(K, V)>>,
}

impl<K, V> fmt::Debug for Drain<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(slice_iter(self.iter.as_slice()))
            .finish()
    }
}

impl<K, V> Drop for Drain<'_, K, V> {
    fn drop(&mut self) {
        for pair in &mut self.iter {
            unsafe { pair.assume_init_drop() };
        }
    }
}

impl<K, V> Iterator for Drain<'_, K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        self.iter.next().map(|p| unsafe { p.assume_init_read() })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.len(), Some(self.iter.len()))
    }
}

impl<K, V> ExactSizeIterator for Drain<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<K, V> FusedIterator for Drain<'_, K, V> {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/drain/tests_101.rs"));
