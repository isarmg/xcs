// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;
use core::fmt;
use core::fmt::Write;

impl<K, V, const N: usize> fmt::Display for Map<K, V, N>
where
    K: fmt::Display,
    V: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('{')?;
        let mut it = self.iter();
        if let Some((k, v)) = it.next() {
            write!(f, "{k}: {v}")?;
            it.try_for_each(|(k, v)| write!(f, ", {k}: {v}"))?;
        }
        f.write_char('}')?;
        Ok(())
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/display/tests_25.rs"));
