// Copyright 2013-2014 The Rust Project Developers.
// Copyright 2018 The Uuid Project Developers.
//
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Generate and parse universally unique identifiers (UUIDs).
//!
//! Here's an example of a UUID:
//!
//! ```text
//! 67e55044-10b1-426f-9247-bb680e5fe0c8
//! ```
//!
//! A UUID is a unique 128-bit value, stored as 16 octets, and regularly
//! formatted as a hex string in five groups. UUIDs are used to assign unique
//! identifiers to entities without requiring a central allocating authority.
//!
//! They are particularly useful in distributed systems, though can be used in
//! disparate areas, such as databases and network protocols.  Typically a UUID
//! is displayed in a readable string form as a sequence of hexadecimal digits,
//! separated into groups by hyphens.
//!
//! The uniqueness property is not strictly guaranteed, however for all
//! practical purposes, it can be assumed that an unintentional collision would
//! be extremely unlikely.
//!
//! UUIDs have a number of standardized encodings that are specified in [RFC 9562](https://www.ietf.org/rfc/rfc9562.html).
//!
//! # Getting started
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies.uuid]
//! version = "1.28.0"
//! # Lets you generate random UUIDs
//! features = [
//!     "v4",
//! ]
//! ```
//!
//! When you want a UUID, you can generate one:
//!
//! ```
//! # fn main() {
//! # #[cfg(feature = "v4")]
//! # {
//! use uuid::Uuid;
//!
//! let id = Uuid::new_v4();
//! # }
//! # }
//! ```
//!
//! If you have a UUID value, you can use its string literal form inline:
//!
//! ```
//! use uuid::{uuid, Uuid};
//!
//! const ID: Uuid = uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8");
//! ```
//!
//! # Working with different UUID versions
//!
//! This library supports all standardized methods for generating UUIDs through individual Cargo features.
//!
//! By default, this crate depends on nothing but the Rust standard library and can parse and format
//! UUIDs, but cannot generate them. Depending on the kind of UUID you'd like to work with, there
//! are Cargo features that enable generating them:
//!
//! * `v1` - Version 1 UUIDs using a timestamp and monotonic counter.
//! * `v3` - Version 3 UUIDs based on the MD5 hash of some data.
//! * `v4` - Version 4 UUIDs with random data.
//! * `v5` - Version 5 UUIDs based on the SHA1 hash of some data.
//! * `v6` - Version 6 UUIDs using a timestamp and monotonic counter.
//! * `v7` - Version 7 UUIDs using a Unix timestamp.
//! * `v8` - Version 8 UUIDs using user-defined data.
//!
//! This library also includes a [`Builder`] type that can be used to help construct UUIDs of any
//! version without any additional dependencies or features. It's a lower-level API than [`Uuid`]
//! that can be used when you need control over implicit requirements on things like a source
//! of randomness.
//!
//! ## Which UUID version should I use?
//!
//! If you just want to generate unique identifiers then consider version 4 (`v4`) UUIDs. If you want
//! to use UUIDs as database keys or need to sort them then consider version 7 (`v7`) UUIDs.
//! Other versions should generally be avoided unless there's an existing need for them.
//!
//! Some UUID versions supersede others. Prefer version 6 over version 1 and version 5 over version 3.
//!
//! # Other features
//!
//! Other crate features can also be useful beyond the version support:
//!
//! * `serde` - adds the ability to serialize and deserialize a UUID using
//!   `serde`.
//! * `borsh` - adds the ability to serialize and deserialize a UUID using
//!   `borsh`.
//! * `arbitrary` - adds an `Arbitrary` trait implementation to `Uuid` for
//!   fuzzing.
//! * `fast-rng` - uses a faster algorithm for generating random UUIDs when available.
//!   This feature requires more dependencies to compile, but is just as suitable for
//!   UUIDs as the default algorithm.
//! * `rng-rand` - forces `rand` as the backend for randomness.
//! * `rng-system-entropy` - forces `getrandom` as the backend for randomness.
//! * `bytemuck` - adds a `Pod` trait implementation to `Uuid` for byte manipulation
//!
//! # Unstable features
//!
//! Some features are unstable. They may be incomplete or depend on other
//! unstable libraries. These include:
//!
//! * `zerocopy` - adds support for zero-copy deserialization using the
//!   `zerocopy` library.
//!
//! Unstable features may break between minor releases.
//!
//! To allow unstable features, you'll need to enable the Cargo feature as
//! normal, but also pass an additional flag through your environment to opt-in
//! to unstable `uuid` features:
//!
//! ```text
//! RUSTFLAGS="--cfg uuid_unstable"
//! ```
//!
//! # Building for other targets
//!
//! ## WebAssembly
//!
//! For WebAssembly, enable the `js` feature:
//!
//! ```toml
//! [dependencies.uuid]
//! version = "1.28.0"
//! features = [
//!     "v4",
//!     "v7",
//!     "js",
//! ]
//! ```
//!
//! ## Embedded
//!
//! For embedded targets without the standard library, you'll need to
//! disable default features when building `uuid`:
//!
//! ```toml
//! [dependencies.uuid]
//! version = "1.28.0"
//! default-features = false
//! ```
//!
//! Some additional features are supported in no-std environments:
//!
//! * `v1`, `v3`, `v5`, `v6`, and `v8`.
//! * `serde`.
//!
//! If you need to use `v4` or `v7` in a no-std environment, you'll need to
//! produce random bytes yourself and then pass them to [`Builder::from_random_bytes`]
//! without enabling the `v4` or `v7` features.
//!
//! If you're using `getrandom`, you can specify the `rng-system-entropy` or `rng-rand`
//! features of `uuid` and configure `getrandom`'s provider per its docs. `uuid`
//! may upgrade its version of `getrandom` in minor releases.
//!
//! # Examples
//!
//! Parse a UUID given in the simple format and print it as a URN:
//!
//! ```
//! # use uuid::Uuid;
//! # fn main() -> Result<(), uuid::Error> {
//! let my_uuid = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
//!
//! println!("{}", my_uuid.urn());
//! # Ok(())
//! # }
//! ```
//!
//! Generate a random UUID and print it out in hexadecimal form:
//!
//! ```
//! // Note that this requires the `v4` feature to be enabled.
//! # use uuid::Uuid;
//! # fn main() {
//! # #[cfg(feature = "v4")] {
//! let my_uuid = Uuid::new_v4();
//!
//! println!("{}", my_uuid);
//! # }
//! # }
//! ```
//!
//! # References
//!
//! * [Wikipedia: Universally Unique Identifier](http://en.wikipedia.org/wiki/Universally_unique_identifier)
//! * [RFC 9562: Universally Unique IDentifiers (UUID)](https://www.ietf.org/rfc/rfc9562.html).
//!
//! [`wasm-bindgen`]: https://crates.io/crates/wasm-bindgen

#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]
#![deny(missing_debug_implementations, missing_docs)]
#![allow(clippy::mixed_attributes_style)]
#![doc(
    html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
    html_favicon_url = "https://www.rust-lang.org/favicon.ico",
    html_root_url = "https://docs.rs/uuid/1.28.0"
)]

#[cfg(any(feature = "std", test))]
#[macro_use]
extern crate std;

#[cfg(all(not(feature = "std"), not(test)))]
#[macro_use]
extern crate core as std;

#[macro_use]
mod macros;

mod builder;
mod error;
mod non_nil;
mod parser;

pub mod fmt;
pub mod timestamp;

use core::hash::{Hash, Hasher};
pub use timestamp::{context::NoContext, ClockSequence, Timestamp};

#[cfg(any(feature = "v1", feature = "v6"))]
#[allow(deprecated)]
pub use timestamp::context::Context;

#[cfg(any(feature = "v1", feature = "v6"))]
pub use timestamp::context::ContextV1;

#[cfg(feature = "v7")]
pub use timestamp::context::ContextV7;

#[cfg(feature = "v1")]
#[doc(hidden)]
// Soft-deprecated (Rust doesn't support deprecating re-exports)
// Use `Context` from the crate root instead
pub mod v1;
#[cfg(feature = "v3")]
mod v3;
#[cfg(feature = "v4")]
mod v4;
#[cfg(feature = "v5")]
mod v5;
#[cfg(feature = "v6")]
mod v6;
#[cfg(feature = "v7")]
mod v7;
#[cfg(feature = "v8")]
mod v8;

#[cfg(feature = "md5")]
mod md5;
#[cfg(feature = "rng")]
mod rng;
#[cfg(feature = "sha1")]
mod sha1;

mod external;

#[doc(hidden)]
pub mod __macro_support {
    pub use crate::std::result::Result::{Err, Ok};
}

pub use crate::{builder::Builder, error::Error, non_nil::NonNilUuid};

/// A 128-bit (16 byte) buffer containing the UUID.
///
/// # ABI
///
/// The `Bytes` type is always guaranteed to be have the same ABI as [`Uuid`].
pub type Bytes = [u8; 16];

/// The version of the UUID, denoting the generating algorithm.
///
/// # References
///
/// * [Version Field in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-4.2)
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
#[repr(u8)]
pub enum Version {
    /// The "nil" (all zeros) UUID.
    Nil = 0u8,
    /// Version 1: Timestamp and node ID.
    Mac = 1,
    /// Version 2: DCE Security.
    Dce = 2,
    /// Version 3: MD5 hash.
    Md5 = 3,
    /// Version 4: Random.
    Random = 4,
    /// Version 5: SHA-1 hash.
    Sha1 = 5,
    /// Version 6: Sortable Timestamp and node ID.
    SortMac = 6,
    /// Version 7: Timestamp and random.
    SortRand = 7,
    /// Version 8: Custom.
    Custom = 8,
    /// The "max" (all ones) UUID.
    Max = 0x0f,
}

/// The reserved variants of UUIDs.
///
/// Unlike the version field, which is a strict set of values, the variant
/// behaves more like a mask. Multiple bit patterns in a UUID's variant field may correspond
/// to the same variant value.
///
/// # References
///
/// * [Variant Field in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-4.1)
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
#[repr(u8)]
pub enum Variant {
    /// Reserved by the NCS for backward compatibility.
    ///
    /// The Nil UUID will return this variant.
    NCS = 0u8,
    /// The variant specified in RFC9562.
    ///
    /// The majority of UUIDs use this variant.
    RFC4122,
    /// Reserved by Microsoft for backward compatibility.
    Microsoft,
    /// Reserved for future expansion.
    ///
    /// The Max UUID will return this variant.
    Future,
}

/// A Universally Unique Identifier (UUID).
///
/// # Examples
///
/// Parse a UUID given in the simple format and print it as a urn:
///
/// ```
/// # use uuid::Uuid;
/// # fn main() -> Result<(), uuid::Error> {
/// let my_uuid = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
///
/// println!("{}", my_uuid.urn());
/// # Ok(())
/// # }
/// ```
///
/// Create a new random (V4) UUID and print it out in hexadecimal form:
///
/// ```
/// // Note that this requires the `v4` feature enabled in the uuid crate.
/// # use uuid::Uuid;
/// # fn main() {
/// # #[cfg(feature = "v4")] {
/// let my_uuid = Uuid::new_v4();
///
/// println!("{}", my_uuid);
/// # }
/// # }
/// ```
///
/// # Formatting
///
/// A UUID can be formatted in one of a few ways:
///
/// * [`simple`](#method.simple): `a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8`.
/// * [`hyphenated`](#method.hyphenated):
///   `a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8`.
/// * [`urn`](#method.urn): `urn:uuid:A1A2A3A4-B1B2-C1C2-D1D2-D3D4D5D6D7D8`.
/// * [`braced`](#method.braced): `{a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8}`.
///
/// The default representation when formatting a UUID with `Display` is
/// hyphenated:
///
/// ```
/// # use uuid::Uuid;
/// # fn main() -> Result<(), uuid::Error> {
/// let my_uuid = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
///
/// assert_eq!(
///     "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
///     my_uuid.to_string(),
/// );
/// # Ok(())
/// # }
/// ```
///
/// Other formats can be specified using adapter methods on the UUID:
///
/// ```
/// # use uuid::Uuid;
/// # fn main() -> Result<(), uuid::Error> {
/// let my_uuid = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
///
/// assert_eq!(
///     "urn:uuid:a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8",
///     my_uuid.urn().to_string(),
/// );
/// # Ok(())
/// # }
/// ```
///
/// # Endianness
///
/// The specification for UUIDs encodes the integer fields that make up the
/// value in big-endian order. This crate assumes integer inputs are already in
/// the correct order by default, regardless of the endianness of the
/// environment. Most methods that accept integers have a `_le` variant (such as
/// `from_fields_le`) that assumes any integer values will need to have their
/// bytes flipped, regardless of the endianness of the environment.
///
/// Most users won't need to worry about endianness unless they need to operate
/// on individual fields (such as when converting between Microsoft GUIDs). The
/// important things to remember are:
///
/// - The endianness is in terms of the fields of the UUID, not the environment.
/// - The endianness is assumed to be big-endian when there's no `_le` suffix
///   somewhere.
/// - Byte-flipping in `_le` methods applies to each integer.
/// - Endianness roundtrips, so if you create a UUID with `from_fields_le`
///   you'll get the same values back out with `to_fields_le`.
///
/// # ABI
///
/// The `Uuid` type is always guaranteed to be have the same ABI as [`Bytes`].
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
// NOTE: Also check `NonNilUuid` when ading new derives here
#[cfg_attr(
    feature = "borsh",
    derive(borsh_derive::BorshDeserialize, borsh_derive::BorshSerialize)
)]


pub struct Uuid(Bytes);

impl Uuid {
    /// UUID namespace for Domain Name System (DNS).
    pub const NAMESPACE_DNS: Self = Uuid([
        0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// UUID namespace for ISO Object Identifiers (OIDs).
    pub const NAMESPACE_OID: Self = Uuid([
        0x6b, 0xa7, 0xb8, 0x12, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// UUID namespace for Uniform Resource Locators (URLs).
    pub const NAMESPACE_URL: Self = Uuid([
        0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// UUID namespace for X.500 Distinguished Names (DNs).
    pub const NAMESPACE_X500: Self = Uuid([
        0x6b, 0xa7, 0xb8, 0x14, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// Returns the variant of the UUID structure.
    ///
    /// This determines the interpretation of the structure of the UUID.
    /// This method simply reads the value of the variant byte. It doesn't
    /// validate the rest of the UUID as conforming to that variant.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::{Uuid, Variant};
    /// # fn main() -> Result<(), uuid::Error> {
    /// let my_uuid = Uuid::parse_str("02f09a3f-1624-3b1d-8409-44eff7708208")?;
    ///
    /// assert_eq!(Variant::RFC4122, my_uuid.get_variant());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # References
    ///
    /// * [Variant Field in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-4.1)
    pub const fn get_variant(&self) -> Variant {
        match self.as_bytes()[8] {
            x if x & 0x80 == 0x00 => Variant::NCS,
            x if x & 0xc0 == 0x80 => Variant::RFC4122,
            x if x & 0xe0 == 0xc0 => Variant::Microsoft,
            x if x & 0xe0 == 0xe0 => Variant::Future,
            // The above match arms are actually exhaustive
            // We just return `Future` here because we can't
            // use `unreachable!()` in a `const fn`
            _ => Variant::Future,
        }
    }

    /// Returns the version number of the UUID.
    ///
    /// This represents the algorithm used to generate the value.
    /// This method is the future-proof alternative to [`Uuid::get_version`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::Uuid;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let my_uuid = Uuid::parse_str("02f09a3f-1624-3b1d-8409-44eff7708208")?;
    ///
    /// assert_eq!(3, my_uuid.get_version_num());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # References
    ///
    /// * [Version Field in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-4.2)
    pub const fn get_version_num(&self) -> usize {
        (self.as_bytes()[6] >> 4) as usize
    }

    /// Returns the version of the UUID.
    ///
    /// This represents the algorithm used to generate the value.
    /// If the version field doesn't contain a recognized version then `None`
    /// is returned. If you're trying to read the version for a future extension
    /// you can also use [`Uuid::get_version_num`] to unconditionally return a
    /// number. Future extensions may start to return `Some` once they're
    /// standardized and supported.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use uuid::{Uuid, Version};
    /// # fn main() -> Result<(), uuid::Error> {
    /// let my_uuid = Uuid::parse_str("02f09a3f-1624-3b1d-8409-44eff7708208")?;
    ///
    /// assert_eq!(Some(Version::Md5), my_uuid.get_version());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # References
    ///
    /// * [Version Field in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-4.2)
    pub const fn get_version(&self) -> Option<Version> {
        match self.get_version_num() {
            0 if self.is_nil() => Some(Version::Nil),
            1 => Some(Version::Mac),
            2 => Some(Version::Dce),
            3 => Some(Version::Md5),
            4 => Some(Version::Random),
            5 => Some(Version::Sha1),
            6 => Some(Version::SortMac),
            7 => Some(Version::SortRand),
            8 => Some(Version::Custom),
            0xf if self.is_max() => Some(Version::Max),
            _ => None,
        }
    }

    /// Returns the four field values of the UUID.
    ///
    /// These values can be passed to the [`Uuid::from_fields`] method to get
    /// the original `Uuid` back.
    ///
    /// * The first field value represents the first group of (eight) hex
    ///   digits, taken as a big-endian `u32` value.  For V1 UUIDs, this field
    ///   represents the low 32 bits of the timestamp.
    /// * The second field value represents the second group of (four) hex
    ///   digits, taken as a big-endian `u16` value.  For V1 UUIDs, this field
    ///   represents the middle 16 bits of the timestamp.
    /// * The third field value represents the third group of (four) hex digits,
    ///   taken as a big-endian `u16` value.  The 4 most significant bits give
    ///   the UUID version, and for V1 UUIDs, the last 12 bits represent the
    ///   high 12 bits of the timestamp.
    /// * The last field value represents the last two groups of four and twelve
    ///   hex digits, taken in order.  The first 1-3 bits of this indicate the
    ///   UUID variant, and for V1 UUIDs, the next 13-15 bits indicate the clock
    ///   sequence and the last 48 bits indicate the node ID.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::nil();
    ///
    /// assert_eq!(uuid.as_fields(), (0, 0, 0, &[0u8; 8]));
    ///
    /// let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8")?;
    ///
    /// assert_eq!(
    ///     uuid.as_fields(),
    ///     (
    ///         0xa1a2a3a4,
    ///         0xb1b2,
    ///         0xc1c2,
    ///         &[0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8],
    ///     )
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_fields(&self) -> (u32, u16, u16, &[u8; 8]) {
        let bytes = self.as_bytes();

        let d1 = (bytes[0] as u32) << 24
            | (bytes[1] as u32) << 16
            | (bytes[2] as u32) << 8
            | (bytes[3] as u32);

        let d2 = (bytes[4] as u16) << 8 | (bytes[5] as u16);

        let d3 = (bytes[6] as u16) << 8 | (bytes[7] as u16);

        let d4: &[u8; 8] = bytes[8..16].try_into().unwrap();
        (d1, d2, d3, d4)
    }

    /// Returns the four field values of the UUID in little-endian order.
    ///
    /// The bytes in the returned integer fields will be converted from
    /// big-endian order. This is based on the endianness of the UUID,
    /// rather than the target environment so bytes will be flipped on both
    /// big and little endian machines.
    ///
    /// # Examples
    ///
    /// ```
    /// use uuid::Uuid;
    ///
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8")?;
    ///
    /// assert_eq!(
    ///     uuid.to_fields_le(),
    ///     (
    ///         0xa4a3a2a1,
    ///         0xb2b1,
    ///         0xc2c1,
    ///         &[0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8],
    ///     )
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_fields_le(&self) -> (u32, u16, u16, &[u8; 8]) {
        let d1 = (self.as_bytes()[0] as u32)
            | (self.as_bytes()[1] as u32) << 8
            | (self.as_bytes()[2] as u32) << 16
            | (self.as_bytes()[3] as u32) << 24;

        let d2 = (self.as_bytes()[4] as u16) | (self.as_bytes()[5] as u16) << 8;

        let d3 = (self.as_bytes()[6] as u16) | (self.as_bytes()[7] as u16) << 8;

        let d4: &[u8; 8] = self.as_bytes()[8..16].try_into().unwrap();
        (d1, d2, d3, d4)
    }

    /// Returns a 128bit value containing the value.
    ///
    /// The bytes in the UUID will be packed directly into a `u128`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8")?;
    ///
    /// assert_eq!(
    ///     uuid.as_u128(),
    ///     0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn as_u128(&self) -> u128 {
        u128::from_be_bytes(*self.as_bytes())
    }

    /// Returns a 128bit little-endian value containing the value.
    ///
    /// The bytes in the `u128` will be flipped to convert into big-endian
    /// order. This is based on the endianness of the UUID, rather than the
    /// target environment so bytes will be flipped on both big and little
    /// endian machines.
    ///
    /// Note that this will produce a different result than
    /// [`Uuid::to_fields_le`], because the entire UUID is reversed, rather
    /// than reversing the individual fields in-place.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8")?;
    ///
    /// assert_eq!(
    ///     uuid.to_u128_le(),
    ///     0xd8d7d6d5d4d3d2d1c2c1b2b1a4a3a2a1,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn to_u128_le(&self) -> u128 {
        u128::from_le_bytes(*self.as_bytes())
    }

    /// Returns two 64bit values containing the value.
    ///
    /// The bytes in the UUID will be split into two `u64`.
    /// The first u64 represents the 64 most significant bits,
    /// the second one represents the 64 least significant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8")?;
    /// assert_eq!(
    ///     uuid.as_u64_pair(),
    ///     (0xa1a2a3a4b1b2c1c2, 0xd1d2d3d4d5d6d7d8),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn as_u64_pair(&self) -> (u64, u64) {
        let value = self.as_u128();
        ((value >> 64) as u64, value as u64)
    }

    /// Returns a slice of 16 octets containing the value.
    ///
    /// This method borrows the underlying byte value of the UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let bytes1 = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    /// let uuid1 = Uuid::from_bytes_ref(&bytes1);
    ///
    /// let bytes2 = uuid1.as_bytes();
    /// let uuid2 = Uuid::from_bytes_ref(bytes2);
    ///
    /// assert_eq!(uuid1, uuid2);
    ///
    /// assert!(std::ptr::eq(
    ///     uuid2 as *const Uuid as *const u8,
    ///     &bytes1 as *const [u8; 16] as *const u8,
    /// ));
    /// ```
    #[inline]
    pub const fn as_bytes(&self) -> &Bytes {
        &self.0
    }

    /// Consumes self and returns the underlying byte value of the UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let bytes = [
    ///     0xa1, 0xa2, 0xa3, 0xa4,
    ///     0xb1, 0xb2,
    ///     0xc1, 0xc2,
    ///     0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    /// ];
    /// let uuid = Uuid::from_bytes(bytes);
    /// assert_eq!(bytes, uuid.into_bytes());
    /// ```
    #[inline]
    pub const fn into_bytes(self) -> Bytes {
        self.0
    }

    /// Returns the bytes of the UUID in little-endian order.
    ///
    /// The bytes for each field will be flipped to convert into little-endian order.
    /// This is based on the endianness of the UUID, rather than the target environment
    /// so bytes will be flipped on both big and little endian machines.
    ///
    /// Note that ordering is applied to each _field_, rather than to the bytes as a whole.
    /// This ordering is compatible with Microsoft's mixed endian GUID format.
    ///
    /// # Examples
    ///
    /// ```
    /// use uuid::Uuid;
    ///
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8")?;
    ///
    /// assert_eq!(
    ///     uuid.to_bytes_le(),
    ///     ([
    ///         0xa4, 0xa3, 0xa2, 0xa1, 0xb2, 0xb1, 0xc2, 0xc1, 0xd1, 0xd2,
    ///         0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8
    ///     ])
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub const fn to_bytes_le(&self) -> Bytes {
        [
            self.0[3], self.0[2], self.0[1], self.0[0], self.0[5], self.0[4], self.0[7], self.0[6],
            self.0[8], self.0[9], self.0[10], self.0[11], self.0[12], self.0[13], self.0[14],
            self.0[15],
        ]
    }

    /// Tests if the UUID is nil (all zeros).
    pub const fn is_nil(&self) -> bool {
        self.as_u128() == u128::MIN
    }

    /// Tests if the UUID is max (all ones).
    pub const fn is_max(&self) -> bool {
        self.as_u128() == u128::MAX
    }

    /// A buffer that can be used for `encode_...` calls, that is
    /// guaranteed to be long enough for any of the format adapters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use uuid::Uuid;
    /// let uuid = Uuid::nil();
    ///
    /// assert_eq!(
    ///     uuid.simple().encode_lower(&mut Uuid::encode_buffer()),
    ///     "00000000000000000000000000000000"
    /// );
    ///
    /// assert_eq!(
    ///     uuid.hyphenated()
    ///         .encode_lower(&mut Uuid::encode_buffer()),
    ///     "00000000-0000-0000-0000-000000000000"
    /// );
    ///
    /// assert_eq!(
    ///     uuid.urn().encode_lower(&mut Uuid::encode_buffer()),
    ///     "urn:uuid:00000000-0000-0000-0000-000000000000"
    /// );
    /// ```
    pub const fn encode_buffer() -> [u8; fmt::Urn::LENGTH] {
        [0; fmt::Urn::LENGTH]
    }

    /// If the UUID is the correct version (v1, v6, or v7) this will return
    /// the timestamp in a version-agnostic [`Timestamp`]. For other versions
    /// this will return `None`.
    ///
    /// # Roundtripping
    ///
    /// This method is unlikely to roundtrip a timestamp in a UUID due to the way
    /// UUIDs encode timestamps. The timestamp returned from this method will be truncated to
    /// 100ns precision for version 1 and 6 UUIDs, and to millisecond precision for version 7 UUIDs.
    pub const fn get_timestamp(&self) -> Option<Timestamp> {
        match self.get_version() {
            Some(Version::Mac) => {
                let (ticks, counter) = timestamp::decode_gregorian_timestamp(self);

                Some(Timestamp::from_gregorian_time(ticks, counter))
            }
            Some(Version::SortMac) => {
                let (ticks, counter) = timestamp::decode_sorted_gregorian_timestamp(self);

                Some(Timestamp::from_gregorian_time(ticks, counter))
            }
            Some(Version::SortRand) => {
                let millis = timestamp::decode_unix_timestamp_millis(self);

                let seconds = millis / 1000;
                let nanos = ((millis % 1000) * 1_000_000) as u32;

                Some(Timestamp::from_unix_time(seconds, nanos, 0, 0))
            }
            _ => None,
        }
    }

    /// If the UUID is the correct version (v1, or v6) this will return the
    /// node value as a 6-byte array. For other versions this will return `None`.
    pub const fn get_node_id(&self) -> Option<[u8; 6]> {
        match self.get_version() {
            Some(Version::Mac) | Some(Version::SortMac) => {
                let mut node_id = [0; 6];

                node_id[0] = self.0[10];
                node_id[1] = self.0[11];
                node_id[2] = self.0[12];
                node_id[3] = self.0[13];
                node_id[4] = self.0[14];
                node_id[5] = self.0[15];

                Some(node_id)
            }
            _ => None,
        }
    }
}

impl Hash for Uuid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0);
    }
}

impl Default for Uuid {
    #[inline]
    fn default() -> Self {
        Uuid::nil()
    }
}

impl AsRef<Uuid> for Uuid {
    #[inline]
    fn as_ref(&self) -> &Uuid {
        self
    }
}

impl AsRef<[u8]> for Uuid {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(feature = "std")]
impl From<Uuid> for std::vec::Vec<u8> {
    fn from(value: Uuid) -> Self {
        value.0.to_vec()
    }
}

#[cfg(feature = "std")]
impl TryFrom<std::vec::Vec<u8>> for Uuid {
    type Error = Error;

    fn try_from(value: std::vec::Vec<u8>) -> Result<Self, Self::Error> {
        Uuid::from_slice(&value)
    }
}

#[cfg(feature = "serialization")]
pub mod data_serialization {
    //! Adapters for alternative `serde` formats.
    //!
    //! This module contains adapters you can use with [`#[serialization(with)]`](https://serde.rs/field-attrs.html#with)
    //! to change the way a [`Uuid`](../struct.Uuid.html) is serialized
    //! and deserialized.

    pub use crate::external::serialization_support::{braced, bytes, compact, hyphenated, simple, urn};
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/uuid/unit/src/lib/tests_993.rs"));
