// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use super::{MutableZeroVecLike, ZeroMap, ZeroMapBorrowed, ZeroMapKV, ZeroVecLike};
use core::fmt;
use core::marker::PhantomData;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
#[cfg(feature = "serde")]
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
#[cfg(feature = "serde")]
impl<'a, K, V> Serialize for ZeroMap<'a, K, V>
where
    K: ZeroMapKV<'a> + Serialize + ?Sized + Ord,
    V: ZeroMapKV<'a> + Serialize + ?Sized,
    K::Container: Serialize,
    V::Container: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Many human-readable formats don't support values other
            // than numbers and strings as map keys. For them, we can serialize
            // as a vec of tuples instead
            if let Some(k) = self.iter_keys().next() {
                if !K::Container::zvl_get_as_t(k, super::serde_helpers::is_num_or_string) {
                    let mut seq = serializer.serialize_seq(Some(self.len()))?;
                    for (k, v) in self.iter() {
                        K::Container::zvl_get_as_t(k, |k| {
                            V::Container::zvl_get_as_t(v, |v| seq.serialize_element(&(k, v)))
                        })?;
                    }
                    return seq.end();
                }
            }
            let mut map = serializer.serialize_map(Some(self.len()))?;
            for (k, v) in self.iter() {
                K::Container::zvl_get_as_t(k, |k| map.serialize_key(k))?;
                V::Container::zvl_get_as_t(v, |v| map.serialize_value(v))?;
            }
            map.end()
        } else {
            (&self.keys, &self.values).serialize(serializer)
        }
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
#[cfg(feature = "serde")]
impl<'a, K, V> Serialize for ZeroMapBorrowed<'a, K, V>
where
    K: ZeroMapKV<'a> + Serialize + ?Sized + Ord,
    V: ZeroMapKV<'a> + Serialize + ?Sized,
    K::Container: Serialize,
    V::Container: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ZeroMap::<K, V>::from(*self).serialize(serializer)
    }
}

/// Modified example from <https://serde.rs/deserialize-map.html>
struct ZeroMapMapVisitor<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized + Ord,
    V: ZeroMapKV<'a> + ?Sized,
{
    #[expect(clippy::type_complexity)] // it's a marker type, complexity doesn't matter
    marker: PhantomData<fn() -> (&'a K::OwnedType, &'a V::OwnedType)>,
}

impl<'a, K, V> ZeroMapMapVisitor<'a, K, V>
where
    K: ZeroMapKV<'a> + ?Sized + Ord,
    V: ZeroMapKV<'a> + ?Sized,
{
    fn new() -> Self {
        ZeroMapMapVisitor {
            marker: PhantomData,
        }
    }
}

impl<'a, 'de, K, V> Visitor<'de> for ZeroMapMapVisitor<'a, K, V>
where
    K: ZeroMapKV<'a> + Ord + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    K::OwnedType: Deserialize<'de>,
    V::OwnedType: Deserialize<'de>,
{
    type Value = ZeroMap<'a, K, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map produced by ZeroMap")
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut map = ZeroMap::with_capacity(access.size_hint().unwrap_or(0));

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_element::<(K::OwnedType, V::OwnedType)>()? {
            // Try to append it at the end, hoping for a sorted map.
            // If not sorted, return an error
            // a serialized map that came from another ZeroMap
            if map
                .try_append(
                    K::Container::owned_as_t(&key),
                    V::Container::owned_as_t(&value),
                )
                .is_some()
            {
                return Err(de::Error::custom(
                    "ZeroMap's keys must be sorted while deserializing",
                ));
            }
        }

        Ok(map)
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = ZeroMap::with_capacity(access.size_hint().unwrap_or(0));

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_entry::<K::OwnedType, V::OwnedType>()? {
            // Try to append it at the end, hoping for a sorted map.
            // If not sorted, return an error
            // a serialized map that came from another ZeroMap
            if map
                .try_append(
                    K::Container::owned_as_t(&key),
                    V::Container::owned_as_t(&value),
                )
                .is_some()
            {
                return Err(de::Error::custom(
                    "ZeroMap's keys must be sorted while deserializing",
                ));
            }
        }

        Ok(map)
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
impl<'de, 'a, K, V> Deserialize<'de> for ZeroMap<'a, K, V>
where
    K: ZeroMapKV<'a> + Ord + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    K::Container: Deserialize<'de>,
    V::Container: Deserialize<'de>,
    K::OwnedType: Deserialize<'de>,
    V::OwnedType: Deserialize<'de>,
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(ZeroMapMapVisitor::<'a, K, V>::new())
        } else {
            let (keys, values): (K::Container, V::Container) =
                Deserialize::deserialize(deserializer)?;
            if keys.zvl_len() != values.zvl_len() {
                return Err(de::Error::custom(
                    "Mismatched key and value sizes in ZeroMap",
                ));
            }
            // #1433: If keys are out of order, treat it as GIGO.
            debug_assert!(keys.zvl_is_ascending());
            Ok(Self { keys, values })
        }
    }
}

// /// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
impl<'de, 'a, K, V> Deserialize<'de> for ZeroMapBorrowed<'a, K, V>
where
    K: ZeroMapKV<'a> + Ord + ?Sized,
    V: ZeroMapKV<'a> + ?Sized,
    K::Container: Deserialize<'de>,
    V::Container: Deserialize<'de>,
    K::OwnedType: Deserialize<'de>,
    V::OwnedType: Deserialize<'de>,
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            Err(de::Error::custom(
                "ZeroMapBorrowed cannot be deserialized from human-readable formats",
            ))
        } else {
            let deserialized: ZeroMap<'a, K, V> = ZeroMap::deserialize(deserializer)?;
            let keys = if let Some(keys) = deserialized.keys.zvl_as_borrowed_inner() {
                keys
            } else {
                return Err(de::Error::custom(
                    "ZeroMapBorrowed can only deserialize in zero-copy ways",
                ));
            };
            let values = if let Some(values) = deserialized.values.zvl_as_borrowed_inner() {
                values
            } else {
                return Err(de::Error::custom(
                    "ZeroMapBorrowed can only deserialize in zero-copy ways",
                ));
            };
            Ok(Self { keys, values })
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerovec/unit/src/map/serde/test_233.rs"));
