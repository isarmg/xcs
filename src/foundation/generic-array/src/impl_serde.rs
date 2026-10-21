//! Serde serialization/deserialization implementation

use core::fmt;
use core::marker::PhantomData;
use serde::de::{self, SeqAccess, Visitor};
use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serialize, Serializer};
use {ArrayLength, GenericArray};

impl<T, N> Serialize for GenericArray<T, N>
where
    T: Serialize,
    N: ArrayLength<T>,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(N::USIZE)?;
        for el in self {
            tup.serialize_element(el)?;
        }

        tup.end()
    }
}

struct GAVisitor<T, N> {
    _t: PhantomData<T>,
    _n: PhantomData<N>,
}

impl<'de, T, N> Visitor<'de> for GAVisitor<T, N>
where
    T: Deserialize<'de> + Default,
    N: ArrayLength<T>,
{
    type Value = GenericArray<T, N>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct GenericArray")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<GenericArray<T, N>, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut result = GenericArray::default();
        for i in 0..N::USIZE {
            result[i] = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(i, &self))?;
        }
        Ok(result)
    }
}

impl<'de, T, N> Deserialize<'de> for GenericArray<T, N>
where
    T: Deserialize<'de> + Default,
    N: ArrayLength<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<GenericArray<T, N>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = GAVisitor {
            _t: PhantomData,
            _n: PhantomData,
        };
        deserializer.deserialize_tuple(N::USIZE, visitor)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/generic-array/unit/src/impl_serde/tests_75.rs"));
