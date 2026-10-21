// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-License-Identifier: MIT

//! A small Set implemented as a Linear Map where the value is `()`.

mod bitand;
mod clone;
mod ctors;
mod debug;
mod difference;
mod display;
mod drain;
mod eq;
mod extend;
mod from;
mod intersection;
mod iterators;
mod methods;
#[cfg(feature = "serialization")]
#[cfg_attr(docsrs, doc(cfg(feature = "serialization")))]
mod serialization;
mod sub;
mod symmetric_difference;
mod union;
// mod bitor; // need nightly Rust to enable `#![feature(generic_const_exprs)]`
// mod bitxor; // need nightly Rust to enable `#![feature(generic_const_exprs)]`

// re-export
pub use difference::Difference;
pub use drain::Drain;
pub use intersection::Intersection;
pub use iterators::{IntoIter, Iter};
pub use symmetric_difference::SymmetricDifference;
pub use union::Union;

use crate::map::Map;

/// A faster alternative of [`std::collections::HashSet`].
///
/// For example, this is how you make a set, which is allocated on stack and is capable of storing
/// up to eight key-values pairs:
///
/// ```
/// use micromap::Set;
/// let mut set : Set<u64, 8> = Set::new();
/// set.insert(1);
/// set.insert(2);
/// assert_eq!(set.len(), 2);
/// assert_eq!(set.capacity(), 8);
/// ```
///
/// It is faster because it doesn't use a hash function at all. It simply keeps
/// all items in an array and when it's necessary to find a value, it goes through
/// all items comparing the needle with each pair available. Also it is faster
/// because it doesn't use heap. When a [`Set`] is being created, it allocates the
/// necessary space on stack. That's why the maximum size of the set must be
/// provided in compile time.
///
/// It is also faster because it doesn't grow in size. When a [`Set`] is created,
/// its size is fixed on stack (or as `Box<Set<_>>` on heap if you like). If an
/// attempt is made to [`insert()`] too many value into it, it simply panics (or
/// you can use [`checked_insert()`]). Moreover, for method [`insert_unchecked()`]
/// in the "release" mode, it doesn't panic, instead, **its behaviour is undefined**.
/// In the "release" mode all boundary checks are disabled, for the sake of higher
/// performance (But in general, you can just use [`insert()`] or [`checked_insert()`]
/// in most cases).
///
/// [`insert()`]: Set::insert
/// [`checked_insert()`]: Set::checked_insert
/// [`insert_unchecked()`]: Set::insert_unchecked
#[repr(transparent)]
pub struct Set<T, const N: usize> {
    map: Map<T, (), N>,
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/micromap/unit/src/set/tests_76.rs"));
