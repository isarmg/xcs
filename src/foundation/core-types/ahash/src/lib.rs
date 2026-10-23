//! AHash is a high performance keyed hash function.
//!
//! It quickly provides a high quality hash where the result is not predictable without knowing the Key.
//! AHash works with `HashMap` to hash keys, but without allowing for the possibility that an malicious user can
//! induce a collision.
//!
//! # How aHash works
//!
//! When it is available aHash uses the hardware AES instructions to provide a keyed hash function.
//! When it is not, aHash falls back on a slightly slower alternative algorithm.
//!
//! Because aHash does not have a fixed standard for its output, it is able to improve over time.
//! But this also means that different computers or computers using different versions of ahash may observe different
//! hash values for the same input.
#![cfg_attr(all(feature = "std",any(feature = "runtime-rng", feature = "no-rng")), doc = r##"
# Basic Usage
AHash provides an implementation of the [Hasher] trait.
To construct a HashMap using aHash as its hasher do the following:
```
use ahash::{AHasher, RandomState};
use std::collections::HashMap;

let mut map: HashMap<i32, i32, RandomState> = HashMap::default();
map.insert(12, 34);
```

### Randomness

The above requires a source of randomness to generate keys for the hashmap. By default this obtained from the OS.
Keys can also be supplied manually with `RandomState::with_seeds`.

### If randomness is not available

[AHasher::default()] can be used to hash using fixed keys. This works with
[BuildHasherDefault](std::hash::BuildHasherDefault). For example:

```
use std::hash::BuildHasherDefault;
use std::collections::HashMap;
use ahash::AHasher;

let mut m: HashMap<_, _, BuildHasherDefault<AHasher>> = HashMap::default();
 # m.insert(12, 34);
```
It is also possible to instantiate [RandomState] directly:

```
use ahash::HashMap;
use ahash::RandomState;

let mut m = HashMap::with_hasher(RandomState::with_seed(42));
 # m.insert(1, 2);
```
Or for uses besides a hashhmap:
```
use std::hash::BuildHasher;
use ahash::RandomState;

let hash_builder = RandomState::with_seed(42);
let hash = hash_builder.hash_one("Some Data");
```
There are several constructors for [RandomState] with different ways to supply seeds.

# Convenience wrappers

For convenience, both new-type wrappers and type aliases are provided.

The new type wrappers are called called `AHashMap` and `AHashSet`.
```
use ahash::AHashMap;

let mut map: AHashMap<i32, i32> = AHashMap::new();
map.insert(12, 34);
```
This avoids the need to type "RandomState". (For convenience `From`, `Into`, and `Deref` are provided).

# Aliases

For even less typing and better interop with existing libraries (such as rayon) which require a `std::collection::HashMap` ,
the type aliases [HashMap], [HashSet] are provided.

```
use ahash::{HashMap, HashMapExt};

let mut map: HashMap<i32, i32> = HashMap::new();
map.insert(12, 34);
```
Note the import of [HashMapExt]. This is needed for the constructor.

"##)]
#![deny(clippy::correctness, clippy::complexity, clippy::perf)]
#![allow(clippy::pedantic, clippy::cast_lossless, clippy::unreadable_literal)]
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

#[macro_use]
mod convert;

#[cfg(any(test, not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "aes", not(miri)))))]
mod fallback_hash;

cfg_if::cfg_if! {
    if #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "aes", not(miri)))] {
        mod aes_hash;
        pub use crate::aes_hash::AHasher;
    } else {
        pub use crate::fallback_hash::AHasher;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        mod hash_map;
        mod hash_set;

        pub use crate::hash_map::AHashMap;
        pub use crate::hash_set::AHashSet;

        /// [Hasher]: std::hash::Hasher
        /// [HashMap]: std::collections::HashMap
        /// Type alias for [HashMap]<K, V, ahash::RandomState>
        pub type HashMap<K, V> = std::collections::HashMap<K, V, crate::RandomState>;

        /// Type alias for [HashSet]<K, ahash::RandomState>
        pub type HashSet<K> = std::collections::HashSet<K, crate::RandomState>;
    }
}

#[cfg(test)]
#[path = "../../../../../test/foundation/ahash/modules/src/hash_quality_test.rs"]
mod hash_quality_test;

mod operations;
pub mod random_state;
#[cfg(test)]
#[path = "../../../../../test/foundation/ahash/modules/src/hash_tests.rs"]
mod hash_tests;

pub use crate::random_state::RandomState;

use core::hash::BuildHasher;

#[cfg(feature = "std")]
/// A convenience trait that can be used together with the type aliases defined to
/// get access to the `new()` and `with_capacity()` methods for the HashMap type alias.
pub trait HashMapExt {
    /// Constructs a new HashMap
    fn new() -> Self;
    /// Constructs a new HashMap with a given initial capacity
    fn with_capacity(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
/// A convenience trait that can be used together with the type aliases defined to
/// get access to the `new()` and `with_capacity()` methods for the HashSet type aliases.
pub trait HashSetExt {
    /// Constructs a new HashSet
    fn new() -> Self;
    /// Constructs a new HashSet with a given initial capacity
    fn with_capacity(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
impl<K, V, S> HashMapExt for std::collections::HashMap<K, V, S>
where
    S: BuildHasher + Default,
{
    fn new() -> Self {
        std::collections::HashMap::with_hasher(S::default())
    }

    fn with_capacity(capacity: usize) -> Self {
        std::collections::HashMap::with_capacity_and_hasher(capacity, S::default())
    }
}

#[cfg(feature = "std")]
impl<K, S> HashSetExt for std::collections::HashSet<K, S>
where
    S: BuildHasher + Default,
{
    fn new() -> Self {
        std::collections::HashSet::with_hasher(S::default())
    }

    fn with_capacity(capacity: usize) -> Self {
        std::collections::HashSet::with_capacity_and_hasher(capacity, S::default())
    }
}

/// Provides a default [Hasher] with fixed keys.
/// This is typically used in conjunction with [BuildHasherDefault] to create
/// [AHasher]s in order to hash the keys of the map.
///
/// Generally it is preferable to use [RandomState] instead, so that different
/// hashmaps will have different keys. However if fixed keys are desirable this
/// may be used instead.
///
/// # Example
/// ```
/// use std::hash::BuildHasherDefault;
/// use ahash::{AHasher, RandomState};
/// use std::collections::HashMap;
///
/// let mut map: HashMap<i32, i32, BuildHasherDefault<AHasher>> = HashMap::default();
/// map.insert(12, 34);
/// ```
///
/// [BuildHasherDefault]: std::hash::BuildHasherDefault
/// [Hasher]: std::hash::Hasher
/// [HashMap]: std::collections::HashMap
impl Default for AHasher {
    /// Constructs a new [AHasher] with fixed keys.
    /// With `runtime-rng`, keys are generated once from OS randomness.
    /// Otherwise hardcoded constants will be used.
    ///
    /// Because the values are fixed, different hashers will all hash elements the same way.
    /// This could make hash values predictable, if DOS attacks are a concern. If this behaviour is
    /// not required, it may be preferable to use [RandomState] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use ahash::AHasher;
    /// use std::hash::Hasher;
    ///
    /// let mut hasher_1 = AHasher::default();
    /// let mut hasher_2 = AHasher::default();
    ///
    /// hasher_1.write_u32(1234);
    /// hasher_2.write_u32(1234);
    ///
    /// assert_eq!(hasher_1.finish(), hasher_2.finish());
    /// ```
    #[inline]
    fn default() -> AHasher {
        RandomState::with_fixed_keys().build_hasher()
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/ahash/unit/src/lib/test_239.rs"));
