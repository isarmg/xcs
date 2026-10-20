// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unicode 16 normalization algorithms integrated into the ZXS naming policy.
//!
//! This private module retains the adopted algorithm variants and their original
//! regression suite. Public callers use the naming operations in the parent crate;
//! upstream licenses and source identities are centralized under `compliance/`.

#![deny(missing_docs, unsafe_code)]
#![allow(dead_code, unused_imports)]

pub use crate::unicode::casefold::nfc_casefold;
pub use crate::unicode::decompose::Decompositions;
pub use crate::unicode::quick_check::{
    IsNormalized, is_nfc, is_nfc_quick, is_nfc_stream_safe, is_nfc_stream_safe_quick, is_nfd,
    is_nfd_quick, is_nfd_stream_safe, is_nfd_stream_safe_quick, is_nfkc, is_nfkc_quick, is_nfkd,
    is_nfkd_quick,
};
pub use crate::unicode::recompose::Recompositions;
pub use crate::unicode::replace::Replacements;
pub use crate::unicode::stream_safe::StreamSafe;
pub use crate::unicode::tables::UNICODE_VERSION;
use core::{option, str::Chars};

mod casefold;
mod decompose;
mod lookups;
mod normalize;
mod perfect_hash;
mod quick_check;
mod recompose;
mod replace;
mod stream_safe;
mod tables;

#[cfg(test)]
#[doc(hidden)]
pub mod __test_api;
#[cfg(test)]
mod test;

/// Methods for composing and decomposing characters.
pub mod char {
    pub use crate::unicode::normalize::{
        compose, decompose_canonical, decompose_cjk_compat_variants, decompose_compatible,
    };

    pub use crate::unicode::lookups::{canonical_combining_class, is_combining_mark};

    /// Return whether the given character is assigned (`General_Category` != `Unassigned`)
    /// and not Private-Use (`General_Category` != `Private_Use`), in the supported version
    /// of Unicode.
    pub use crate::unicode::tables::is_public_assigned;
}

/// Methods for iterating over strings while applying Unicode normalizations
/// as described in
/// [Unicode Standard Annex #15](http://www.unicode.org/reports/tr15/).
pub trait UnicodeNormalization<I: Iterator<Item = char>> {
    /// Returns an iterator over the string in Unicode Normalization Form D
    /// (canonical decomposition).
    fn nfd(self) -> Decompositions<I>;

    /// Returns an iterator over the string in Unicode Normalization Form KD
    /// (compatibility decomposition).
    fn nfkd(self) -> Decompositions<I>;

    /// An Iterator over the string in Unicode Normalization Form C
    /// (canonical decomposition followed by canonical composition).
    fn nfc(self) -> Recompositions<I>;

    /// An Iterator over the string in Unicode Normalization Form KC
    /// (compatibility decomposition followed by canonical composition).
    fn nfkc(self) -> Recompositions<I>;

    /// A transformation which replaces CJK Compatibility Ideograph codepoints
    /// with normal forms using Standardized Variation Sequences. This is not
    /// part of the canonical or compatibility decomposition algorithms, but
    /// performing it before those algorithms produces normalized output which
    /// better preserves the intent of the original text.
    ///
    /// Note that many systems today ignore variation selectors, so these
    /// may not immediately help text display as intended, but they at
    /// least preserve the information in a standardized form, giving
    /// implementations the option to recognize them.
    fn cjk_compat_variants(self) -> Replacements<I>;

    /// An Iterator over the string with Conjoining Grapheme Joiner characters
    /// inserted according to the Stream-Safe Text Process (UAX15-D4)
    fn stream_safe(self) -> StreamSafe<I>;
}

impl<'a> UnicodeNormalization<Chars<'a>> for &'a str {
    #[inline]
    fn nfd(self) -> Decompositions<Chars<'a>> {
        Decompositions::new_canonical(self.chars())
    }

    #[inline]
    fn nfkd(self) -> Decompositions<Chars<'a>> {
        Decompositions::new_compatible(self.chars())
    }

    #[inline]
    fn nfc(self) -> Recompositions<Chars<'a>> {
        Recompositions::new_canonical(self.chars())
    }

    #[inline]
    fn nfkc(self) -> Recompositions<Chars<'a>> {
        Recompositions::new_compatible(self.chars())
    }

    #[inline]
    fn cjk_compat_variants(self) -> Replacements<Chars<'a>> {
        replace::new_cjk_compat_variants(self.chars())
    }

    #[inline]
    fn stream_safe(self) -> StreamSafe<Chars<'a>> {
        StreamSafe::new(self.chars())
    }
}

impl UnicodeNormalization<option::IntoIter<char>> for char {
    #[inline]
    fn nfd(self) -> Decompositions<option::IntoIter<char>> {
        Decompositions::new_canonical(Some(self).into_iter())
    }

    #[inline]
    fn nfkd(self) -> Decompositions<option::IntoIter<char>> {
        Decompositions::new_compatible(Some(self).into_iter())
    }

    #[inline]
    fn nfc(self) -> Recompositions<option::IntoIter<char>> {
        Recompositions::new_canonical(Some(self).into_iter())
    }

    #[inline]
    fn nfkc(self) -> Recompositions<option::IntoIter<char>> {
        Recompositions::new_compatible(Some(self).into_iter())
    }

    #[inline]
    fn cjk_compat_variants(self) -> Replacements<option::IntoIter<char>> {
        replace::new_cjk_compat_variants(Some(self).into_iter())
    }

    #[inline]
    fn stream_safe(self) -> StreamSafe<option::IntoIter<char>> {
        StreamSafe::new(Some(self).into_iter())
    }
}

impl<I: Iterator<Item = char>> UnicodeNormalization<I> for I {
    #[inline]
    fn nfd(self) -> Decompositions<I> {
        Decompositions::new_canonical(self)
    }

    #[inline]
    fn nfkd(self) -> Decompositions<I> {
        Decompositions::new_compatible(self)
    }

    #[inline]
    fn nfc(self) -> Recompositions<I> {
        Recompositions::new_canonical(self)
    }

    #[inline]
    fn nfkc(self) -> Recompositions<I> {
        Recompositions::new_compatible(self)
    }

    #[inline]
    fn cjk_compat_variants(self) -> Replacements<I> {
        replace::new_cjk_compat_variants(self)
    }

    #[inline]
    fn stream_safe(self) -> StreamSafe<I> {
        StreamSafe::new(self)
    }
}
