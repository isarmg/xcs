// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::options::ZeroTrieWithOptions;
use crate::zerotrie::ZeroTrieFlavor;
use crate::ZeroAsciiIgnoreCaseTrie;
use crate::ZeroTrie;
use crate::ZeroTrieExtendedCapacity;
use crate::ZeroTriePerfectHash;
use crate::ZeroTrieSimpleAscii;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use litemap::LiteMap;
use data_serialization::de::Error;
use data_serialization::de::Visitor;
use data_serialization::Deserialize;
use data_serialization::Deserializer;
use data_serialization::Serialize;
use data_serialization::Serializer;

struct ByteStrVisitor;
impl<'de> Visitor<'de> for ByteStrVisitor {
    type Value = Box<[u8]>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a slice of borrowed bytes or a string")
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(Box::from(v))
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Box::from(v.as_bytes()))
    }
    fn visit_seq<A>(self, mut v: A) -> Result<Self::Value, A::Error>
    where
        A: data_serialization::de::SeqAccess<'de>,
    {
        let mut result = Vec::with_capacity(v.size_hint().unwrap_or(0));
        while let Some(x) = v.next_element::<u8>()? {
            result.push(x);
        }
        Ok(Box::from(result))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct SerializedByteString(pub(crate) Box<[u8]>);

impl SerializedByteString {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SerializedByteString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = deserializer.deserialize_any(ByteStrVisitor)?;
            Ok(SerializedByteString(s))
        } else {
            let s = Vec::<u8>::deserialize(deserializer)?;
            Ok(SerializedByteString(s.into_boxed_slice()))
        }
    }
}

impl Serialize for SerializedByteString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes: &[u8] = &self.0;
        if serializer.is_human_readable() {
            if let Ok(s) = core::str::from_utf8(bytes) {
                return serializer.serialize_str(s);
            }
        }
        serializer.serialize_bytes(bytes)
    }
}

struct SerializedByteSlice<'a>(&'a [u8]);

impl Serialize for SerializedByteSlice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            if let Ok(s) = core::str::from_utf8(self.0) {
                return serializer.serialize_str(s);
            }
        }
        serializer.serialize_bytes(self.0)
    }
}

impl<'data, 'de: 'data, Store> Deserialize<'de> for ZeroTrieSimpleAscii<Store>
where
    // DISCUSS: There are several possibilities for the bounds here that would
    // get the job done. I could look for Deserialize, but this would require
    // creating a custom Deserializer for the map case. I also considered
    // introducing a new trait instead of relying on From.
    Store: From<&'data [u8]> + From<Vec<u8>> + 'data,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let lm = LiteMap::<SerializedByteString, usize>::deserialize(deserializer)?;
            ZeroTrieSimpleAscii::try_from_serialization_litemap(&lm)
                .map_err(D::Error::custom)
                .map(|trie| trie.convert_store())
        } else {
            // Note: `impl Deserialize for &[u8]` uses visit_borrowed_bytes
            let (flags, trie_bytes) = <(u8, &[u8])>::deserialize(deserializer)?;
            if Self::OPTIONS.to_u8_flags() != flags {
                return Err(D::Error::custom("invalid ZeroTrie tag"));
            };
            Ok(ZeroTrieSimpleAscii::from_store(Store::from(trie_bytes)))
        }
    }
}

impl<Store> Serialize for ZeroTrieSimpleAscii<Store>
where
    Store: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let lm = self.to_litemap_serialization();
            lm.serialize(serializer)
        } else {
            (Self::FLAGS, SerializedByteSlice(self.as_bytes())).serialize(serializer)
        }
    }
}

impl<'de, 'data, Store> Deserialize<'de> for ZeroAsciiIgnoreCaseTrie<Store>
where
    'de: 'data,
    // DISCUSS: There are several possibilities for the bounds here that would
    // get the job done. I could look for Deserialize, but this would require
    // creating a custom Deserializer for the map case. I also considered
    // introducing a new trait instead of relying on From.
    Store: From<&'data [u8]> + From<Vec<u8>> + 'data,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let lm = LiteMap::<SerializedByteString, usize>::deserialize(deserializer)?;
            ZeroAsciiIgnoreCaseTrie::try_from_serialization_litemap(&lm)
                .map_err(D::Error::custom)
                .map(|trie| trie.convert_store())
        } else {
            // Note: `impl Deserialize for &[u8]` uses visit_borrowed_bytes
            let (flags, trie_bytes) = <(u8, &[u8])>::deserialize(deserializer)?;
            if Self::OPTIONS.to_u8_flags() != flags {
                return Err(D::Error::custom("invalid ZeroTrie tag"));
            }
            Ok(ZeroAsciiIgnoreCaseTrie::from_store(Store::from(trie_bytes)))
        }
    }
}

impl<Store> Serialize for ZeroAsciiIgnoreCaseTrie<Store>
where
    Store: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let lm = self.to_litemap_serialization();
            lm.serialize(serializer)
        } else {
            (Self::OPTIONS.to_u8_flags(), SerializedByteSlice(self.as_bytes())).serialize(serializer)
        }
    }
}

impl<'de, 'data, Store> Deserialize<'de> for ZeroTriePerfectHash<Store>
where
    'de: 'data,
    Store: From<&'data [u8]> + From<Vec<u8>> + 'data,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let lm = LiteMap::<SerializedByteString, usize>::deserialize(deserializer)?;
            ZeroTriePerfectHash::try_from_serialization_litemap(&lm)
                .map_err(D::Error::custom)
                .map(|trie| trie.convert_store())
        } else {
            // Note: `impl Deserialize for &[u8]` uses visit_borrowed_bytes
            let (flags, trie_bytes) = <(u8, &[u8])>::deserialize(deserializer)?;
            if Self::OPTIONS.to_u8_flags() != flags {
                return Err(D::Error::custom("invalid ZeroTrie tag"));
            }
            Ok(ZeroTriePerfectHash::from_store(Store::from(trie_bytes)))
        }
    }
}

impl<Store> Serialize for ZeroTriePerfectHash<Store>
where
    Store: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let lm = self.to_litemap_serialization();
            lm.serialize(serializer)
        } else {
            (Self::OPTIONS.to_u8_flags(), SerializedByteSlice(self.as_bytes())).serialize(serializer)
        }
    }
}

impl<'de, 'data, Store> Deserialize<'de> for ZeroTrieExtendedCapacity<Store>
where
    'de: 'data,
    Store: From<&'data [u8]> + From<Vec<u8>> + 'data,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let lm = LiteMap::<SerializedByteString, usize>::deserialize(deserializer)?;
            ZeroTrieExtendedCapacity::try_from_serialization_litemap(&lm)
                .map_err(D::Error::custom)
                .map(|trie| trie.convert_store())
        } else {
            // Note: `impl Deserialize for &[u8]` uses visit_borrowed_bytes
            let (flags, trie_bytes) = <(u8, &[u8])>::deserialize(deserializer)?;
            if Self::OPTIONS.to_u8_flags() != flags {
                return Err(D::Error::custom("invalid ZeroTrie tag"));
            }
            Ok(ZeroTrieExtendedCapacity::from_store(Store::from(
                trie_bytes,
            )))
        }
    }
}

impl<Store> Serialize for ZeroTrieExtendedCapacity<Store>
where
    Store: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let lm = self.to_litemap_serialization();
            lm.serialize(serializer)
        } else {
            (Self::OPTIONS.to_u8_flags(), SerializedByteSlice(self.as_bytes())).serialize(serializer)
        }
    }
}

impl<'de, 'data, Store> Deserialize<'de> for ZeroTrie<Store>
where
    'de: 'data,
    Store: From<&'data [u8]> + From<Vec<u8>> + 'data,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let lm = LiteMap::<SerializedByteString, usize>::deserialize(deserializer)?;
            ZeroTrie::<Vec<u8>>::try_from(&lm)
                .map_err(D::Error::custom)
                .map(|trie| trie.convert_store())
        } else {
            // Note: `impl Deserialize for &[u8]` uses visit_borrowed_bytes
            let bytes = <&[u8]>::deserialize(deserializer)?;
            let (tag, trie_bytes) = bytes
                .split_first()
                .ok_or_else(|| D::Error::custom("expected at least 1 byte for ZeroTrie"))?;
            let store = Store::from(trie_bytes);
            let zerotrie = if *tag == ZeroTrieSimpleAscii::<u8>::OPTIONS.to_u8_flags() {
                ZeroTrieSimpleAscii::from_store(store).into_zerotrie()
            } else if *tag == ZeroTriePerfectHash::<u8>::OPTIONS.to_u8_flags() {
                ZeroTriePerfectHash::from_store(store).into_zerotrie()
            } else if *tag == ZeroTrieExtendedCapacity::<u8>::OPTIONS.to_u8_flags() {
                ZeroTrieExtendedCapacity::from_store(store).into_zerotrie()
            } else {
                return Err(D::Error::custom("invalid ZeroTrie tag"));
            };
            Ok(zerotrie)
        }
    }
}

impl<Store> Serialize for ZeroTrie<Store>
where
    Store: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let lm = self.to_litemap_serialization();
            lm.serialize(serializer)
        } else {
            let (tag, bytes) = match &self.0 {
                ZeroTrieFlavor::SimpleAscii(t) => (
                    ZeroTrieSimpleAscii::<u8>::OPTIONS.to_u8_flags(),
                    t.as_bytes(),
                ),
                ZeroTrieFlavor::PerfectHash(t) => (
                    ZeroTriePerfectHash::<u8>::OPTIONS.to_u8_flags(),
                    t.as_bytes(),
                ),
                ZeroTrieFlavor::ExtendedCapacity(t) => (
                    ZeroTrieExtendedCapacity::<u8>::OPTIONS.to_u8_flags(),
                    t.as_bytes(),
                ),
            };
            let mut all_in_one_vec = Vec::with_capacity(bytes.len() + 1);
            all_in_one_vec.push(tag);
            all_in_one_vec.extend(bytes);
            serializer.serialize_bytes(&all_in_one_vec)
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerotrie/unit/src/data_serialization/testdata_349.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerotrie/unit/src/data_serialization/tests_354.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerotrie/unit/src/data_serialization/tests_zerovec_565.rs"));
