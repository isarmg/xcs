// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;

impl<K, V, const N: usize> Clone for Map<K, V, N>
where
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        let mut m = Self::new();
        m.len = self.len;
        m.pairs
            .iter_mut()
            .zip(self.pairs[..self.len].iter())
            .for_each(|(dst, src)| unsafe {
                dst.write(src.assume_init_ref().clone());
            });
        m
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/clone/tests_24.rs"));
