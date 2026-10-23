//! Specialized serialization for flags types using `serde`.

use crate::{
    parser::{self, ParseHex, WriteHex},
    Flags,
};
use core::{fmt, str};
use data_serialization::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

/**
Serialize a set of flags as a human-readable string or their underlying bits.

Any unknown bits will be retained.
*/
pub fn serialize<B: Flags, S: Serializer>(flags: &B, serializer: S) -> Result<S::Ok, S::Error>
where
    B::Bits: WriteHex + Serialize,
{
    // Serialize human-readable flags as a string like `"A | B"`
    if serializer.is_human_readable() {
        serializer.collect_str(&parser::AsDisplay(flags))
    }
    // Serialize non-human-readable flags directly as the underlying bits
    else {
        flags.bits().serialize(serializer)
    }
}

/**
Deserialize a set of flags from a human-readable string or their underlying bits.

Any unknown bits will be retained.
*/
pub fn deserialize<'de, B: Flags, D: Deserializer<'de>>(deserializer: D) -> Result<B, D::Error>
where
    B::Bits: ParseHex + Deserialize<'de>,
{
    if deserializer.is_human_readable() {
        // Deserialize human-readable flags by parsing them from strings like `"A | B"`
        struct FlagsVisitor<B>(core::marker::PhantomData<B>);

        impl<'de, B: Flags> Visitor<'de> for FlagsVisitor<B>
        where
            B::Bits: ParseHex,
        {
            type Value = B;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string value of `|` separated flags")
            }

            fn visit_str<E: Error>(self, flags: &str) -> Result<Self::Value, E> {
                parser::from_str(flags).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_str(FlagsVisitor(Default::default()))
    } else {
        // Deserialize non-human-readable flags directly from the underlying bits
        let bits = B::Bits::deserialize(deserializer)?;

        Ok(B::from_bits_retain(bits))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/flag-set/unit/src/external/data_serialization/tests_69.rs"));
