// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::{LanguageIdentifier, Locale};
use core::{fmt::Display, marker::PhantomData, str::FromStr};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use writeable::Writeable;

impl Serialize for LanguageIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.write_to_string())
    }
}

impl Serialize for Locale {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.write_to_string())
    }
}

struct ParseVisitor<T>(PhantomData<T>);

impl<T> serde::de::Visitor<'_> for ParseVisitor<T>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    type Value = T;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "a valid Unicode Language or Locale Identifier")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        s.parse::<T>().map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for LanguageIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ParseVisitor(PhantomData))
    }
}

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ParseVisitor(PhantomData))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_locale_core/functions/src/serde/json_67.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_locale_core/functions/src/serde/postcard_143.rs"));
