// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

use super::Set;
use core::fmt::Formatter;
use core::marker::PhantomData;
use data_serialization::de::{SeqAccess, Visitor};
use data_serialization::ser::SerializeSeq;
use data_serialization::{Deserialize, Deserializer, Serialize, Serializer};

impl<T: Serialize, const N: usize> Serialize for Set<T, N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for k in self {
            seq.serialize_element(k)?;
        }
        seq.end()
    }
}

struct Vi<T, const N: usize>(PhantomData<T>);

impl<'de, T: PartialEq + Deserialize<'de>, const N: usize> Visitor<'de> for Vi<T, N> {
    type Value = Set<T, N>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a Set")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut m: Self::Value = Set::new();
        while let Some(key) = seq.next_element()? {
            m.insert(key);
        }
        Ok(m)
    }
}

impl<'de, T: PartialEq + Deserialize<'de>, const N: usize> Deserialize<'de> for Set<T, N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(Vi(PhantomData))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/serialization/tests_54.rs"));
