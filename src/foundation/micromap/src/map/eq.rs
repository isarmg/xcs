// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;

impl<K: PartialEq, V: PartialEq, const N: usize, const M: usize> PartialEq<Map<K, V, M>>
    for Map<K, V, N>
{
    /// Two maps can be compared. (The capacity does not affect comparison.)
    ///
    /// # Examples
    /// ```
    /// let mut m1: micromap::Map<u8, i32, 5> = micromap::Map::new();
    /// let mut m2: micromap::Map<u8, i32, 10> = micromap::Map::new();
    /// m1.insert(1, 42);
    /// m2.insert(1, 42);
    /// assert_eq!(m1, m2);
    /// // two maps with different order of key-value pairs are still equal:
    /// m1.insert(2, 1);
    /// m1.insert(3, 16);
    /// m2.insert(3, 16);
    /// m2.insert(2, 1);
    /// assert_eq!(m1, m2);
    /// ```
    #[inline]
    fn eq(&self, other: &Map<K, V, M>) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K: Eq, V: Eq, const N: usize> Eq for Map<K, V, N> {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/eq/tests_33.rs"));
