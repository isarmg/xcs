// ZXS-maintained extension to unicode-normalization 0.1.24.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Original normalization code and its notices remain in the adjacent modules.

use crate::unicode::UnicodeNormalization;
use core::{option, str::Chars};

enum FoldedChars {
    Unchanged(option::IntoIter<char>),
    Mapping(Chars<'static>),
}

impl Iterator for FoldedChars {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            Self::Unchanged(iter) => iter.next(),
            Self::Mapping(iter) => iter.next(),
        }
    }
}

/// Stream canonical normalization, a fixed case-fold mapping, then NFC.
///
/// ZXS supplies its versioned Unicode 16 case-fold table through `fold`. `None`
/// preserves a scalar; `Some(mapping)` substitutes the complete mapping, which
/// may contain multiple scalars. This function does not use Rust's host-version
/// casing tables, compatibility normalization, or stream-safe CGJ insertion.
///
/// Both normalization stages are necessary: a mapping may depend on a composed
/// input scalar, and adjacent mapped scalars may compose or require reordering.
/// The stages share an iterator pipeline instead of materializing two complete
/// intermediate strings. Normalization still buffers combining sequences as
/// required by Unicode; this is not a constant-memory guarantee for arbitrary
/// unbounded input. ZXS validates its name/path input budgets separately.
pub fn nfc_casefold<'a, F>(input: &'a str, mut fold: F) -> impl Iterator<Item = char> + 'a
where
    F: FnMut(char) -> Option<&'static str> + 'a,
{
    input
        .nfc()
        .flat_map(move |ch| match fold(ch) {
            Some(mapping) => FoldedChars::Mapping(mapping.chars()),
            None => FoldedChars::Unchanged(Some(ch).into_iter()),
        })
        .nfc()
}
