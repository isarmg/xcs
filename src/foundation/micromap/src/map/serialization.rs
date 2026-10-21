// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Map;
use core::fmt::Formatter;
use core::marker::PhantomData;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl<K: Serialize, V: Serialize, const N: usize> Serialize for Map<K, V, N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (a, v) in self {
            map.serialize_entry(a, v)?;
        }
        map.end()
    }
}

struct Vi<K, V, const N: usize>(PhantomData<K>, PhantomData<V>);

impl<'de, K: PartialEq + Deserialize<'de>, V: Deserialize<'de>, const N: usize> Visitor<'de>
    for Vi<K, V, N>
{
    type Value = Map<K, V, N>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a Map")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut m: Self::Value = Map::new();
        while let Some((key, value)) = access.next_entry()? {
            m.insert(key, value);
        }
        Ok(m)
    }
}

impl<'de, K: PartialEq + Deserialize<'de>, V: Deserialize<'de>, const N: usize> Deserialize<'de>
    for Map<K, V, N>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(Vi(PhantomData, PhantomData))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/map/serialization/tests_58.rs"));
