// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use super::ZeroHashMap;
use crate::{
    map::{ZeroMapKV, ZeroVecLike},
    ZeroVec,
};

use serde::{de, Deserialize, Serialize};

impl<'a, K, V> Serialize for ZeroHashMap<'a, K, V>
where
    K: ZeroMapKV<'a> + Serialize + ?Sized,
    V: ZeroMapKV<'a> + Serialize + ?Sized,
    K::Container: Serialize,
    V::Container: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.displacements, &self.keys, &self.values).serialize(serializer)
    }
}

impl<'de, 'a, K, V> Deserialize<'de> for ZeroHashMap<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    K::Container: Deserialize<'de>,
    V::Container: Deserialize<'de>,
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (displacements, keys, values): (ZeroVec<(u32, u32)>, K::Container, V::Container) =
            Deserialize::deserialize(deserializer)?;
        if keys.zvl_len() != values.zvl_len() {
            return Err(de::Error::custom(
                "Mismatched key and value sizes in ZeroHashMap",
            ));
        }
        if displacements.zvl_len() != keys.zvl_len() {
            return Err(de::Error::custom(
                "Mismatched displacements and key, value sizes in ZeroHashMap",
            ));
        }
        Ok(Self {
            displacements,
            keys,
            values,
        })
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerovec/unit/src/hashmap/serde/test_60.rs"));
