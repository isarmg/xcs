// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use core::{
    char,
    ops::{Bound::*, RangeBounds},
};
use potential_utf::PotentialCodePoint;
use zerovec::ZeroVec;
use zerovec::ule::AsULE;

/// Returns whether the vector is sorted ascending non inclusive, of even length,
/// and within the bounds of `0x0 -> 0x10FFFF + 1` inclusive.
#[expect(clippy::indexing_slicing)] // windows
#[expect(clippy::unwrap_used)] // by is_empty check
pub fn is_valid_zv(inv_list_zv: &ZeroVec<'_, PotentialCodePoint>) -> bool {
    inv_list_zv.is_empty()
        || (inv_list_zv.len().is_multiple_of(2)
            && inv_list_zv.as_ule_slice().windows(2).all(|chunk| {
                <PotentialCodePoint as AsULE>::from_unaligned(chunk[0])
                    < <PotentialCodePoint as AsULE>::from_unaligned(chunk[1])
            })
            && u32::from(inv_list_zv.last().unwrap()) <= char::MAX as u32 + 1)
}

/// Returns start (inclusive) and end (exclusive) bounds of [`RangeBounds`]
pub fn deconstruct_range<T>(range: impl RangeBounds<T>) -> (u32, u32)
where
    T: Into<u32> + Copy,
{
    let from = match range.start_bound() {
        Included(b) => (*b).into(),
        Excluded(_) => unreachable!(),
        Unbounded => 0,
    };
    let till = match range.end_bound() {
        Included(b) => (*b).into() + 1,
        Excluded(b) => (*b).into(),
        Unbounded => (char::MAX as u32) + 1,
    };
    (from, till)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_collections/unit/src/codepointinvlist/utils/tests_45.rs"));
