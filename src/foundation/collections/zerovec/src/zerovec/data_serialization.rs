// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use super::{ZeroSlice, ZeroVec};
use crate::ule::*;
#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use core::marker::PhantomData;
#[cfg(feature = "alloc")]
use data_serialization::de::SeqAccess;
use data_serialization::de::{Deserialize, Deserializer, Error, Visitor};
#[cfg(feature = "serialization")]
use data_serialization::ser::{Serialize, SerializeSeq, Serializer};

struct ZeroVecVisitor<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for ZeroVecVisitor<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de, T> Visitor<'de> for ZeroVecVisitor<T>
where
    T: 'de + Deserialize<'de> + AsULE,
{
    type Value = ZeroVec<'de, T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence or borrowed buffer of fixed-width elements")
    }

    fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        ZeroVec::parse_bytes(bytes).map_err(Error::custom)
    }

    #[cfg(feature = "alloc")]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut vec: Vec<T::ULE> = if let Some(capacity) = seq.size_hint() {
            Vec::with_capacity(capacity)
        } else {
            Vec::new()
        };
        while let Some(value) = seq.next_element::<T>()? {
            vec.push(T::to_unaligned(value));
        }
        Ok(ZeroVec::new_owned(vec))
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
impl<'de, 'a, T> Deserialize<'de> for ZeroVec<'a, T>
where
    T: 'de + Deserialize<'de> + AsULE,
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = ZeroVecVisitor::default();
        if deserializer.is_human_readable() {
            deserializer.deserialize_seq(visitor)
        } else {
            deserializer.deserialize_bytes(visitor)
        }
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
impl<T> Serialize for ZeroVec<'_, T>
where
    T: Serialize + AsULE,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut seq = serializer.serialize_seq(Some(self.len()))?;
            for value in self.iter() {
                seq.serialize_element(&value)?;
            }
            seq.end()
        } else {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
#[cfg(feature = "alloc")]
impl<'de, T> Deserialize<'de> for Box<ZeroSlice<T>>
where
    T: Deserialize<'de> + AsULE + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut zv = ZeroVec::<T>::deserialize(deserializer)?;
        let vec = zv.with_mut(core::mem::take);
        Ok(ZeroSlice::from_boxed_slice(vec.into_boxed_slice()))
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
impl<'de, 'a, T> Deserialize<'de> for &'a ZeroSlice<T>
where
    T: Deserialize<'de> + AsULE + 'static,
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            Err(Error::custom(
                "&ZeroSlice cannot be deserialized from human-readable formats",
            ))
        } else {
            let deserialized: ZeroVec<'a, T> = ZeroVec::deserialize(deserializer)?;
            let borrowed = if let Some(b) = deserialized.as_maybe_borrowed() {
                b
            } else {
                return Err(Error::custom(
                    "&ZeroSlice can only deserialize in zero-copy ways",
                ));
            };
            Ok(borrowed)
        }
    }
}

/// This impl requires enabling the optional `serde` Cargo feature of the `zerovec` crate
impl<T> Serialize for ZeroSlice<T>
where
    T: Serialize + AsULE,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_zerovec().serialize(serializer)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/zerovec/unit/src/zerovec/data_serialization/test_160.rs"));
