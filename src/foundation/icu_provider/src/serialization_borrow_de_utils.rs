// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use alloc::borrow::Cow;
use data_serialization::Deserialize;
use data_serialization::de::Deserializer;

#[derive(Deserialize)]
#[serialization(transparent)]
// Cows fail to borrow in some situations (array, option), but structs of Cows don't.
#[allow(clippy::exhaustive_structs)] // newtype
#[derive(Debug)]
pub struct CowWrap<'data>(#[serialization(borrow)] pub Cow<'data, str>);

#[derive(Deserialize)]
#[serialization(transparent)]
// Cows fail to borrow in some situations (array, option), but structs of Cows don't.
#[allow(clippy::exhaustive_structs)] // newtype
#[derive(Debug)]
pub struct CowBytesWrap<'data>(#[serialization(borrow)] pub Cow<'data, [u8]>);

pub fn array_of_cow<'de, D, const N: usize>(deserializer: D) -> Result<[Cow<'de, str>; N], D::Error>
where
    D: Deserializer<'de>,
    [CowWrap<'de>; N]: Deserialize<'de>,
{
    <[CowWrap<'de>; N]>::deserialize(deserializer).map(|array| array.map(|wrap| wrap.0))
}

pub fn option_of_cow<'de, D>(deserializer: D) -> Result<Option<Cow<'de, str>>, D::Error>
where
    D: Deserializer<'de>,
{
    <Option<CowWrap<'de>>>::deserialize(deserializer).map(|opt| opt.map(|wrap| wrap.0))
}

pub fn tuple_of_cow<'de, D>(deserializer: D) -> Result<(Cow<'de, str>, Cow<'de, str>), D::Error>
where
    D: Deserializer<'de>,
    (CowWrap<'de>, CowWrap<'de>): Deserialize<'de>,
{
    <(CowWrap<'de>, CowWrap<'de>)>::deserialize(deserializer).map(|x| (x.0.0, x.1.0))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_provider/functions/src/serialization_borrow_de_utils/test_option_46.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_provider/functions/src/serialization_borrow_de_utils/test_tuple_58.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_provider/functions/src/serialization_borrow_de_utils/test_array_72.rs"));
