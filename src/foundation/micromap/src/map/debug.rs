// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;
use core::fmt;

impl<K, V, const N: usize> fmt::Debug for Map<K, V, N>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/debug/tests_17.rs"));
