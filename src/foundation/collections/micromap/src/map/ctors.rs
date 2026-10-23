// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;
use core::mem::MaybeUninit;

impl<K, V, const N: usize> Default for Map<K, V, N> {
    /// Creates a empty [Map] like [`new()`][`Map::new`].
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, const N: usize> Map<K, V, N> {
    /// Creates an empty (len) Linear Map with capacity `N`.
    ///
    /// The linear map is initially created with a place that has a capacity
    /// of `N` key-value pairs (and one usize), so it will immediately occupy
    /// these memory on the stack (no allocation on heap).
    ///
    /// After creation, capacity will not change any more, which is the max
    /// len of the map.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let map: Map<&str, i32, 20> = Map::new();
    /// assert_eq!(map.capacity(), 20);
    /// assert_eq!(map.len(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            len: 0,
            pairs: [const { MaybeUninit::uninit() }; N],
        }
    }

    /// Creates an empty [Map] with fixed capacity.
    ///
    /// The map will be able to hold at most `capacity` elements. And
    /// the argument `capacity` should be always equal to the generic
    /// constant `N`.
    ///
    /// # Panics
    /// If `capacity` is not equal to `N`, the function will panic.
    ///
    /// # Examples
    /// ```should_panic
    /// # #![allow(deprecated)]
    /// use micromap::Map;
    /// let map: Map<&str, i32, 10> = Map::with_capacity(10); // correct
    /// assert_eq!(map.capacity(), 10);
    /// let map: Map<&str, i32, 10> = Map::with_capacity(20); // panic here
    /// assert_eq!(map.capacity(), 10); // unreachable line
    /// ```
    #[deprecated(note = "Please use `new()` instead.")]
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity == N, "capacity must be equal to N");
        Self::new()
    }
}

impl<K, V, const N: usize> Drop for Map<K, V, N> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe { self.item_drop(i) };
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/micromap/unit/src/map/ctors/tests_76.rs"));
