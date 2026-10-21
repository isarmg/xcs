// Copyright 2013-2014 The Rust Project Developers.
// Copyright 2018 The Uuid Project Developers.
//
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::marker::PhantomData;

use crate::{
    error::*,
    fmt::{Braced, Hyphenated, Simple, Urn},
    non_nil::NonNilUuid,
    std::fmt,
    Bytes, Uuid,
};
use serialization_core::{
    de::{self, Error as _},
    Deserialize, Deserializer, Serialize, Serializer,
};

impl Serialize for Uuid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(self.hyphenated().encode_lower(&mut Uuid::encode_buffer()))
        } else {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
}

impl Serialize for NonNilUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        Uuid::from(*self).serialize(serializer)
    }
}

impl Serialize for Hyphenated {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.encode_lower(&mut Uuid::encode_buffer()))
    }
}

impl Serialize for Simple {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.encode_lower(&mut Uuid::encode_buffer()))
    }
}

impl Serialize for Urn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.encode_lower(&mut Uuid::encode_buffer()))
    }
}

impl Serialize for Braced {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.encode_lower(&mut Uuid::encode_buffer()))
    }
}

struct UuidReadableVisitor<T> {
    expecting: &'static str,
    _marker: PhantomData<T>,
}

impl<'vi, T: UuidDeserialize> de::Visitor<'vi> for UuidReadableVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<T, E> {
        T::from_str(value).map_err(de_error)
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<T, E> {
        T::from_slice(value).map_err(de_error)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<T, A::Error>
    where
        A: de::SeqAccess<'vi>,
    {
        #[rustfmt::skip]
        let bytes = [
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(0, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(1, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(2, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(3, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(4, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(5, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(6, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(7, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(8, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(9, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(10, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(11, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(12, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(13, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(14, &self)) },
            match seq.next_element()? { Some(e) => e, None => return Err(A::Error::invalid_length(15, &self)) },
        ];

        T::from_bytes(bytes).map_err(de_error)
    }
}

struct UuidBytesVisitor<T> {
    _marker: PhantomData<T>,
}

impl<'vi, T: UuidDeserialize> de::Visitor<'vi> for UuidBytesVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a 16 byte array")
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<T, E> {
        T::from_slice(value).map_err(de_error)
    }
}

fn de_error<E: de::Error>(e: Error) -> E {
    E::custom(format_args!("UUID parsing failed: {}", e))
}

trait UuidDeserialize {
    fn from_str(formatted: &str) -> Result<Self, Error>
    where
        Self: Sized;
    fn from_slice(bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;
    fn from_bytes(bytes: Bytes) -> Result<Self, Error>
    where
        Self: Sized;
}

impl UuidDeserialize for Uuid {
    fn from_str(formatted: &str) -> Result<Self, Error> {
        formatted.parse()
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        Uuid::from_slice(bytes)
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        Ok(Uuid::from_bytes(bytes))
    }
}

impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(UuidReadableVisitor {
                expecting: "a formatted UUID string",
                _marker: PhantomData::<Uuid>,
            })
        } else {
            deserializer.deserialize_bytes(UuidBytesVisitor {
                _marker: PhantomData::<Uuid>,
            })
        }
    }
}

impl UuidDeserialize for Braced {
    fn from_str(formatted: &str) -> Result<Self, Error> {
        formatted.parse()
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Uuid::from_slice(bytes)?.into())
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        Ok(Uuid::from_bytes(bytes).into())
    }
}

impl<'de> Deserialize<'de> for Braced {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(UuidReadableVisitor {
            expecting: "a UUID string in the braced format",
            _marker: PhantomData::<Braced>,
        })
    }
}

impl UuidDeserialize for Hyphenated {
    fn from_str(formatted: &str) -> Result<Self, Error> {
        formatted.parse()
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Uuid::from_slice(bytes)?.into())
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        Ok(Uuid::from_bytes(bytes).into())
    }
}

impl<'de> Deserialize<'de> for Hyphenated {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(UuidReadableVisitor {
            expecting: "a UUID string in the hyphenated format",
            _marker: PhantomData::<Hyphenated>,
        })
    }
}

impl UuidDeserialize for Simple {
    fn from_str(formatted: &str) -> Result<Self, Error> {
        formatted.parse()
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Uuid::from_slice(bytes)?.into())
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        Ok(Uuid::from_bytes(bytes).into())
    }
}

impl<'de> Deserialize<'de> for Simple {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(UuidReadableVisitor {
            expecting: "a UUID string in the simple format",
            _marker: PhantomData::<Simple>,
        })
    }
}

impl UuidDeserialize for Urn {
    fn from_str(formatted: &str) -> Result<Self, Error> {
        formatted.parse()
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Uuid::from_slice(bytes)?.into())
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        Ok(Uuid::from_bytes(bytes).into())
    }
}

impl<'de> Deserialize<'de> for Urn {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(UuidReadableVisitor {
            expecting: "a UUID string in the URN format",
            _marker: PhantomData::<Urn>,
        })
    }
}

impl<'de> Deserialize<'de> for NonNilUuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        let uuid = Uuid::deserialize(deserializer)?;

        NonNilUuid::try_from(uuid).map_err(|_| {
            de::Error::invalid_value(de::Unexpected::Other("nil UUID"), &"a non-nil UUID")
        })
    }
}

pub mod compact {
    //! Serialize a [`Uuid`] as a `[u8; 16]`.
    //!
    //! [`Uuid`]: ../../struct.Uuid.html

    /// Serialize from a [`Uuid`] as a `[u8; 16]`
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn serialize<S>(u: &crate::Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        serialization_core::Serialize::serialize(u.as_bytes(), serializer)
    }

    /// Deserialize a `[u8; 16]` as a [`Uuid`]
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn deserialize<'de, D>(deserializer: D) -> Result<crate::Uuid, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        let bytes: [u8; 16] = serialization_core::Deserialize::deserialize(deserializer)?;

        Ok(crate::Uuid::from_bytes(bytes))
    }

    /// Serialize and deserialize an optional [`Uuid`](crate::Uuid) as an optional `[u8; 16]`.
    pub mod option {
        /// Serialize an optional [`Uuid`](crate::Uuid) as an optional `[u8; 16]`.
        pub fn serialize<S>(u: &Option<crate::Uuid>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serialization_core::Serializer,
        {
            match u {
                Some(u) => serializer.serialize_some(u.as_bytes()),
                None => serializer.serialize_none(),
            }
        }

        /// Deserialize an optional `[u8; 16]` as an optional [`Uuid`](crate::Uuid).
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<crate::Uuid>, D::Error>
        where
            D: serialization_core::Deserializer<'de>,
        {
            <Option<[u8; 16]> as serialization_core::Deserialize>::deserialize(deserializer)
                .map(|bytes| bytes.map(crate::Uuid::from_bytes))
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/tests_333.rs"));
}

/// Serialize a [`Uuid`] as a byte string.
///
/// Unlike [`compact`](crate::data_serialization::compact), which serializes a [`Uuid`] as a
/// `[u8; 16]` array, this module uses the serializer's dedicated byte string
/// support through [`serialize_bytes`]. Formats that distinguish byte strings
/// from sequences, such as CBOR or MessagePack, can then encode the [`Uuid`] as
/// a single 16 byte string instead of a sequence of 16 individually encoded
/// integers.
///
/// [`Uuid`]: ../../struct.Uuid.html
/// [`serialize_bytes`]: serialization_core::Serializer::serialize_bytes
///
/// ## Examples
///
/// ```
/// # use uuid::Uuid;
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct Container {
///     #[serialization(with = "uuid::data_serialization::bytes")]
///     id: Uuid,
/// }
/// ```
pub mod bytes {
    use super::*;

    struct ByteUuid(crate::Uuid);

    impl serialization_core::Serialize for ByteUuid {
        fn serialize<S: serialization_core::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(self.0.as_bytes())
        }
    }

    impl<'de> serialization_core::Deserialize<'de> for ByteUuid {
        fn deserialize<D: serialization_core::Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Self, D::Error> {
            deserializer
                .deserialize_bytes(UuidBytesVisitor {
                    _marker: PhantomData::<crate::Uuid>,
                })
                .map(Self)
        }
    }

    /// Serialize from a [`Uuid`] as a byte string.
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn serialize<S>(u: &crate::Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        ByteUuid(*u).serialize(serializer)
    }

    /// Deserialize a byte string as a [`Uuid`].
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn deserialize<'de, D>(deserializer: D) -> Result<crate::Uuid, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        ByteUuid::deserialize(deserializer).map(|u| u.0)
    }

    /// Serialize and deserialize an optional [`Uuid`](crate::Uuid) as an optional byte string.
    pub mod option {
        use super::*;

        /// Serialize an optional [`Uuid`](crate::Uuid) as an optional byte string.
        pub fn serialize<S>(u: &Option<crate::Uuid>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serialization_core::Serializer,
        {
            match u {
                Some(u) => serializer.serialize_some(&ByteUuid(*u)),
                None => serializer.serialize_none(),
            }
        }

        /// Deserialize an optional byte string as an optional [`Uuid`](crate::Uuid).
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<crate::Uuid>, D::Error>
        where
            D: serialization_core::Deserializer<'de>,
        {
            Option::<ByteUuid>::deserialize(deserializer).map(|u| u.map(|u| u.0))
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/tests_534.rs"));
}

/// Serialize a [`Uuid`] as [`uuid::fmt::Simple`](crate::fmt::Simple).
///
/// [`Uuid`]: ../../struct.Uuid.html
///
/// ## Examples
///
/// Serialize and deserialize using the simple format, failing to deserialize
/// any other format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructA {
///     #[serialization(with = "uuid::data_serialization::simple")]
///     id: uuid::Uuid,
/// }
/// ```
///
/// Serialize using the simple format, but deserialize any format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructB {
///     #[serialization(serialize_with = "uuid::data_serialization::simple::serialize")]
///     id: uuid::Uuid,
/// }
/// ```
pub mod simple {
    use super::*;

    /// Serialize a [`Uuid`] as a simple string.
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    ///
    /// # Examples
    ///
    /// ```
    /// #[derive(serialization_derive::Serialize)]
    /// struct Struct {
    ///     #[serialization(serialize_with = "uuid::data_serialization::simple::serialize")]
    ///     id: uuid::Uuid,
    /// }
    ///
    /// ```
    pub fn serialize<S>(u: &crate::Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        serialization_core::Serialize::serialize(u.as_simple(), serializer)
    }

    /// Deserialize a simple-formatted string as a [`Uuid`].
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        Ok(Simple::deserialize(deserializer)?.into())
    }

    /// Serialize and deserialize an optional [`Uuid`](crate::Uuid) in the simple format.
    pub mod option {
        use super::*;

        /// Serialize an optional [`Uuid`](crate::Uuid) in the simple format.
        pub fn serialize<S>(u: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serialization_core::Serializer,
        {
            match u {
                Some(u) => serializer.serialize_some(u.as_simple()),
                None => serializer.serialize_none(),
            }
        }

        /// Deserialize an optional simple string as an optional [`Uuid`](crate::Uuid).
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
        where
            D: serialization_core::Deserializer<'de>,
        {
            Option::<Simple>::deserialize(deserializer).map(|u| u.map(Into::into))
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/tests_694.rs"));
}

/// Serialize a [`Uuid`] as [`uuid::fmt::Braced`](crate::fmt::Braced).
///
/// [`Uuid`]: ../../struct.Uuid.html
///
/// ## Examples
///
/// Serialize and deserialize using the braced format, failing to deserialize
/// any other format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructA {
///     #[serialization(with = "uuid::data_serialization::braced")]
///     id: uuid::Uuid,
/// }
/// ```
///
/// Serialize using the braced format, but deserialize any format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructB {
///     #[serialization(serialize_with = "uuid::data_serialization::braced::serialize")]
///     id: uuid::Uuid,
/// }
/// ```
pub mod braced {
    use super::*;

    /// Serialize a [`Uuid`] as a braced string.
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    ///
    /// # Examples
    ///
    /// ```
    /// #[derive(serialization_derive::Serialize)]
    /// struct Struct {
    ///     #[serialization(serialize_with = "uuid::data_serialization::braced::serialize")]
    ///     id: uuid::Uuid,
    /// }
    ///
    /// ```
    pub fn serialize<S>(u: &crate::Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        serialization_core::Serialize::serialize(u.as_braced(), serializer)
    }

    /// Deserialize a braced-formatted string as a [`Uuid`].
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn deserialize<'de, D>(deserializer: D) -> Result<crate::Uuid, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        Ok(Braced::deserialize(deserializer)?.into())
    }

    /// Serialize and deserialize an optional [`Uuid`](crate::Uuid) in the braced format.
    pub mod option {
        use super::*;

        /// Serialize an optional [`Uuid`](crate::Uuid) in the braced format.
        pub fn serialize<S>(u: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serialization_core::Serializer,
        {
            match u {
                Some(u) => serializer.serialize_some(u.as_braced()),
                None => serializer.serialize_none(),
            }
        }

        /// Deserialize an optional braced string as an optional [`Uuid`](crate::Uuid).
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
        where
            D: serialization_core::Deserializer<'de>,
        {
            Option::<Braced>::deserialize(deserializer).map(|u| u.map(Into::into))
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/tests_881.rs"));
}

/// Serialize a [`Uuid`] as [`uuid::fmt::Hyphenated`](crate::fmt::Hyphenated).
///
/// [`Uuid`]: ../../struct.Uuid.html
///
/// ## Examples
///
/// Serialize and deserialize using the hyphenated format, failing to deserialize
/// any other format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructA {
///     #[serialization(with = "uuid::data_serialization::hyphenated")]
///     id: uuid::Uuid,
/// }
/// ```
///
/// Serialize using the hyphenated format, but deserialize any format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructB {
///     #[serialization(serialize_with = "uuid::data_serialization::hyphenated::serialize")]
///     id: uuid::Uuid,
/// }
/// ```
pub mod hyphenated {

    use super::*;

    /// Serialize a [`Uuid`] as a hyphenated string.
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    ///
    /// # Examples
    ///
    /// ```
    /// #[derive(serialization_derive::Serialize)]
    /// struct Struct {
    ///     #[serialization(serialize_with = "uuid::data_serialization::hyphenated::serialize")]
    ///     id: uuid::Uuid,
    /// }
    ///
    /// ```
    pub fn serialize<S>(u: &crate::Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        serialization_core::Serialize::serialize(u.as_hyphenated(), serializer)
    }

    /// Deserialize a hyphenated-formatted string as a [`Uuid`].
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn deserialize<'de, D>(deserializer: D) -> Result<crate::Uuid, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        Ok(Hyphenated::deserialize(deserializer)?.into())
    }

    /// Serialize and deserialize an optional [`Uuid`](crate::Uuid) in the hyphenated format.
    pub mod option {
        use super::*;

        /// Serialize an optional [`Uuid`](crate::Uuid) in the hyphenated format.
        pub fn serialize<S>(u: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serialization_core::Serializer,
        {
            match u {
                Some(u) => serializer.serialize_some(u.as_hyphenated()),
                None => serializer.serialize_none(),
            }
        }

        /// Deserialize an optional hyphenated string as an optional [`Uuid`](crate::Uuid).
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
        where
            D: serialization_core::Deserializer<'de>,
        {
            Option::<Hyphenated>::deserialize(deserializer).map(|u| u.map(Into::into))
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/tests_1070.rs"));
}

/// Serialize a [`Uuid`] as [`uuid::fmt::Urn`](crate::fmt::Urn).
///
/// [`Uuid`]: ../../struct.Uuid.html
///
/// ## Examples
///
/// Serialize and deserialize using the URN format, failing to deserialize
/// any other format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructA {
///     #[serialization(with = "uuid::data_serialization::urn")]
///     id: uuid::Uuid,
/// }
/// ```
///
/// Serialize using the URN format, but deserialize any format:
///
/// ```
/// #[derive(serialization_derive::Serialize, serialization_derive::Deserialize)]
/// struct StructB {
///     #[serialization(serialize_with = "uuid::data_serialization::urn::serialize")]
///     id: uuid::Uuid,
/// }
/// ```
pub mod urn {
    use super::*;

    /// Serialize a [`Uuid`] as a URN string.
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    ///
    /// # Examples
    ///
    /// ```
    /// #[derive(serialization_derive::Serialize)]
    /// struct Struct {
    ///     #[serialization(serialize_with = "uuid::data_serialization::urn::serialize")]
    ///     id: uuid::Uuid,
    /// }
    ///
    /// ```
    pub fn serialize<S>(u: &crate::Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serialization_core::Serializer,
    {
        serialization_core::Serialize::serialize(u.as_urn(), serializer)
    }

    /// Deserialize a URN-formatted string as a [`Uuid`].
    ///
    /// [`Uuid`]: ../../struct.Uuid.html
    pub fn deserialize<'de, D>(deserializer: D) -> Result<crate::Uuid, D::Error>
    where
        D: serialization_core::Deserializer<'de>,
    {
        Ok(Urn::deserialize(deserializer)?.into())
    }

    /// Serialize and deserialize an optional [`Uuid`](crate::Uuid) in the URN format.
    pub mod option {
        use super::*;

        /// Serialize an optional [`Uuid`](crate::Uuid) in the URN format.
        pub fn serialize<S>(u: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serialization_core::Serializer,
        {
            match u {
                Some(u) => serializer.serialize_some(u.as_urn()),
                None => serializer.serialize_none(),
            }
        }

        /// Deserialize an optional URN string as an optional [`Uuid`](crate::Uuid).
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
        where
            D: serialization_core::Deserializer<'de>,
        {
            Option::<Urn>::deserialize(deserializer).map(|u| u.map(Into::into))
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/tests_1258.rs"));
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/external/serialization_support/serialization_tests_1361.rs"));
