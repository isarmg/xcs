// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use core::iter::FromIterator;
use core::{
    convert::TryFrom,
    ops::{Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};

use super::RangeError;
use crate::codepointinvlist::CodePointInversionList;
use crate::codepointinvlist::CodePointInversionListBuilder;
use crate::codepointinvlist::utils::deconstruct_range;
use potential_utf::PotentialCodePoint;
use zerovec::ZeroVec;

fn try_from_range<'data>(
    range: impl RangeBounds<char>,
) -> Result<CodePointInversionList<'data>, RangeError> {
    let (from, till) = deconstruct_range(range);
    if from < till {
        let set = [
            PotentialCodePoint::from_u24(from),
            PotentialCodePoint::from_u24(till),
        ];
        let inv_list: ZeroVec<PotentialCodePoint> = ZeroVec::alloc_from_slice(&set);
        #[expect(clippy::unwrap_used)] // valid
        Ok(CodePointInversionList::try_from_inversion_list(inv_list).unwrap())
    } else {
        Err(RangeError(from, till))
    }
}

impl TryFrom<Range<char>> for CodePointInversionList<'_> {
    type Error = RangeError;

    fn try_from(range: Range<char>) -> Result<Self, Self::Error> {
        try_from_range(range)
    }
}

impl TryFrom<RangeFrom<char>> for CodePointInversionList<'_> {
    type Error = RangeError;

    fn try_from(range: RangeFrom<char>) -> Result<Self, Self::Error> {
        try_from_range(range)
    }
}

impl TryFrom<RangeFull> for CodePointInversionList<'_> {
    type Error = RangeError;

    fn try_from(_: RangeFull) -> Result<Self, Self::Error> {
        Ok(Self::all())
    }
}

impl TryFrom<RangeInclusive<char>> for CodePointInversionList<'_> {
    type Error = RangeError;

    fn try_from(range: RangeInclusive<char>) -> Result<Self, Self::Error> {
        try_from_range(range)
    }
}

impl TryFrom<RangeTo<char>> for CodePointInversionList<'_> {
    type Error = RangeError;

    fn try_from(range: RangeTo<char>) -> Result<Self, Self::Error> {
        try_from_range(range)
    }
}

impl TryFrom<RangeToInclusive<char>> for CodePointInversionList<'_> {
    type Error = RangeError;

    fn try_from(range: RangeToInclusive<char>) -> Result<Self, Self::Error> {
        try_from_range(range)
    }
}

impl FromIterator<RangeInclusive<u32>> for CodePointInversionList<'_> {
    fn from_iter<I: IntoIterator<Item = RangeInclusive<u32>>>(iter: I) -> Self {
        let mut builder = CodePointInversionListBuilder::new();
        for range in iter {
            builder.add_range32(range);
        }
        builder.build()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/icu_collections/unit/src/codepointinvlist/conversions/tests_93.rs"));
