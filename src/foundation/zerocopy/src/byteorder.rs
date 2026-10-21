// SPDX-License-Identifier: BSD-2-Clause OR Apache-2.0 OR MIT
//
// Copyright 2019 The Fuchsia Authors
//
// Licensed under a BSD-style license <LICENSE-BSD>, Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0>, or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed except according to
// those terms.

//! Byte order-aware numeric primitives.
//!
//! This module contains equivalents of the native multi-byte integer types with
//! no alignment requirement and supporting byte order conversions.
//!
//! For each native multi-byte integer type - `u16`, `i16`, `u32`, etc - and
//! floating point type - `f32` and `f64` - an equivalent type is defined by
//! this module - [`U16`], [`I16`], [`U32`], [`F32`], [`F64`], etc. Unlike their
//! native counterparts, these types have alignment 1, and take a type parameter
//! specifying the byte order in which the bytes are stored in memory. Each type
//! implements this crate's relevant conversion and marker traits.
//!
//! These two properties, taken together, make these types useful for defining
//! data structures whose memory layout matches a wire format such as that of a
//! network protocol or a file format. Such formats often have multi-byte values
//! at offsets that do not respect the alignment requirements of the equivalent
//! native types, and stored in a byte order not necessarily the same as that of
//! the target platform.
//!
//! Type aliases are provided for common byte orders in the [`big_endian`],
//! [`little_endian`], [`network_endian`], and [`native_endian`] submodules.
//! Note that network-endian is a synonym for big-endian.
//!
//! # Example
//!
//! One use of these types is for representing network packet formats, such as
//! UDP:
//!
//! ```rust
//! use zerocopy::{*, byteorder::network_endian::U16};
//! # use zerocopy_derive::*;
//!
//! #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
//! #[repr(C)]
//! struct UdpHeader {
//!     src_port: U16,
//!     dst_port: U16,
//!     length: U16,
//!     checksum: U16,
//! }
//!
//! #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
//! #[repr(C, packed)]
//! struct UdpPacket {
//!     header: UdpHeader,
//!     body: [u8],
//! }
//!
//! impl UdpPacket {
//!     fn parse(bytes: &[u8]) -> Option<&UdpPacket> {
//!         UdpPacket::ref_from_bytes(bytes).ok()
//!     }
//! }
//! ```

use core::{
    convert::{TryFrom, TryInto},
    fmt::{Binary, Debug, LowerHex, Octal, UpperHex},
    hash::Hash,
    num::TryFromIntError,
};

use super::*;

/// A type-level representation of byte order.
///
/// This type is implemented by [`BigEndian`] and [`LittleEndian`], which
/// represent big-endian and little-endian byte order respectively. This module
/// also provides a number of useful aliases for those types: [`NativeEndian`],
/// [`NetworkEndian`], [`BE`], and [`LE`].
///
/// `ByteOrder` types can be used to specify the byte order of the types in this
/// module - for example, [`U32<BigEndian>`] is a 32-bit integer stored in
/// big-endian byte order.
///
/// [`U32<BigEndian>`]: U32
pub trait ByteOrder:
    Copy + Clone + Debug + Display + Eq + PartialEq + Ord + PartialOrd + Hash + private::Sealed
{
    /// A value-level representation of byte order.
    const ORDER: Order;
}

mod private {
    pub trait Sealed {}

    impl Sealed for super::BigEndian {}
    impl Sealed for super::LittleEndian {}
}

/// A value-level representation of [`ByteOrder`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Order {
    /// A value-level representation of [`BigEndian`].
    BigEndian,
    /// A value-level representation of [`LittleEndian`].
    LittleEndian,
}

/// Big-endian byte order.
///
/// See [`ByteOrder`] for more details.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BigEndian {}

impl ByteOrder for BigEndian {
    const ORDER: Order = Order::BigEndian;
}

impl Display for BigEndian {
    #[inline]
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

/// Little-endian byte order.
///
/// See [`ByteOrder`] for more details.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LittleEndian {}

impl ByteOrder for LittleEndian {
    const ORDER: Order = Order::LittleEndian;
}

impl Display for LittleEndian {
    #[inline]
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

/// The endianness used by this platform.
///
/// This is a type alias for [`BigEndian`] or [`LittleEndian`] depending on the
/// endianness of the target platform.
#[cfg(target_endian = "big")]
pub type NativeEndian = BigEndian;

/// The endianness used by this platform.
///
/// This is a type alias for [`BigEndian`] or [`LittleEndian`] depending on the
/// endianness of the target platform.
#[cfg(target_endian = "little")]
pub type NativeEndian = LittleEndian;

/// The endianness used in many network protocols.
///
/// This is a type alias for [`BigEndian`].
pub type NetworkEndian = BigEndian;

/// A type alias for [`BigEndian`].
pub type BE = BigEndian;

/// A type alias for [`LittleEndian`].
pub type LE = LittleEndian;

macro_rules! impl_dbg_trait {
    ($name:ident, $native:ident) => {
        impl<O: ByteOrder> Debug for $name<O> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                // This results in a format like "U16(42)".
                f.debug_tuple(stringify!($name)).field(&self.get()).finish()
            }
        }
    };
}

macro_rules! impl_dbg_traits {
    ($name:ident, $native:ident, "floating point number") => {
        #[cfg(not(no_fp_fmt_parse))]
        impl_dbg_trait!($name, $native);

        #[cfg(no_fp_fmt_parse)]
        impl<O: ByteOrder> Debug for $name<O> {
            #[inline]
            fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
                panic!("floating point support is turned off");
            }
        }
    };
    ($name:ident, $native:ident, "unsigned integer") => {
        impl_dbg_traits!($name, $native, @all_types);
    };
    ($name:ident, $native:ident, "signed integer") => {
        impl_dbg_traits!($name, $native, @all_types);
    };
    ($name:ident, $native:ident, @all_types) => {
        impl_dbg_trait!($name, $native);
    };
}

macro_rules! impl_fmt_trait {
    ($name:ident, $native:ident, $trait:ident) => {
        impl<O: ByteOrder> $trait for $name<O> {
            #[inline(always)]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                $trait::fmt(&self.get(), f)
            }
        }
    };
}

macro_rules! impl_fmt_traits {
    ($name:ident, $native:ident, "floating point number") => {
        #[cfg(not(no_fp_fmt_parse))]
        impl_fmt_trait!($name, $native, Display);
    };
    ($name:ident, $native:ident, "unsigned integer") => {
        impl_fmt_traits!($name, $native, @all_types);
    };
    ($name:ident, $native:ident, "signed integer") => {
        impl_fmt_traits!($name, $native, @all_types);
    };
    ($name:ident, $native:ident, @all_types) => {
        impl_fmt_trait!($name, $native, Display);
        impl_fmt_trait!($name, $native, Octal);
        impl_fmt_trait!($name, $native, LowerHex);
        impl_fmt_trait!($name, $native, UpperHex);
        impl_fmt_trait!($name, $native, Binary);
    };
}

macro_rules! impl_ops_traits {
    ($name:ident, $native:ident, "floating point number") => {
        impl_ops_traits!($name, $native, @all_types);
        impl_ops_traits!($name, $native, @signed_integer_floating_point);

        impl<O: ByteOrder> PartialOrd for $name<O> {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }
    };
    ($name:ident, $native:ident, "unsigned integer") => {
        impl_ops_traits!($name, $native, @signed_unsigned_integer);
        impl_ops_traits!($name, $native, @all_types);
    };
    ($name:ident, $native:ident, "signed integer") => {
        impl_ops_traits!($name, $native, @signed_unsigned_integer);
        impl_ops_traits!($name, $native, @signed_integer_floating_point);
        impl_ops_traits!($name, $native, @all_types);
    };
    ($name:ident, $native:ident, @signed_unsigned_integer) => {
        impl_ops_traits!(@without_byteorder_swap $name, $native, BitAnd, bitand, BitAndAssign, bitand_assign);
        impl_ops_traits!(@without_byteorder_swap $name, $native, BitOr, bitor, BitOrAssign, bitor_assign);
        impl_ops_traits!(@without_byteorder_swap $name, $native, BitXor, bitxor, BitXorAssign, bitxor_assign);
        impl_ops_traits!(@with_byteorder_swap $name, $native, Shl, shl, ShlAssign, shl_assign);
        impl_ops_traits!(@with_byteorder_swap $name, $native, Shr, shr, ShrAssign, shr_assign);

        impl<O> core::ops::Not for $name<O> {
            type Output = $name<O>;

            #[inline(always)]
            fn not(self) -> $name<O> {
                 let self_native = $native::from_ne_bytes(self.0);
                 $name((!self_native).to_ne_bytes(), PhantomData)
            }
        }

        impl<O: ByteOrder> PartialOrd for $name<O> {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<O: ByteOrder> Ord for $name<O> {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl<O: ByteOrder> PartialOrd<$native> for $name<O> {
            #[inline(always)]
            fn partial_cmp(&self, other: &$native) -> Option<Ordering> {
                self.get().partial_cmp(other)
            }
        }
    };
    ($name:ident, $native:ident, @signed_integer_floating_point) => {
        impl<O: ByteOrder> core::ops::Neg for $name<O> {
            type Output = $name<O>;

            #[inline(always)]
            fn neg(self) -> $name<O> {
                let self_native: $native = self.get();
                #[allow(clippy::arithmetic_side_effects)]
                $name::<O>::new(-self_native)
            }
        }
    };
    ($name:ident, $native:ident, @all_types) => {
        impl_ops_traits!(@with_byteorder_swap $name, $native, Add, add, AddAssign, add_assign);
        impl_ops_traits!(@with_byteorder_swap $name, $native, Div, div, DivAssign, div_assign);
        impl_ops_traits!(@with_byteorder_swap $name, $native, Mul, mul, MulAssign, mul_assign);
        impl_ops_traits!(@with_byteorder_swap $name, $native, Rem, rem, RemAssign, rem_assign);
        impl_ops_traits!(@with_byteorder_swap $name, $native, Sub, sub, SubAssign, sub_assign);
    };
    (@with_byteorder_swap $name:ident, $native:ident, $trait:ident, $method:ident, $trait_assign:ident, $method_assign:ident) => {
        impl<O: ByteOrder> core::ops::$trait<$name<O>> for $name<O> {
            type Output = $name<O>;

            #[inline(always)]
            fn $method(self, rhs: $name<O>) -> $name<O> {
                let self_native: $native = self.get();
                let rhs_native: $native = rhs.get();
                let result_native = core::ops::$trait::$method(self_native, rhs_native);
                $name::<O>::new(result_native)
            }
        }

        impl<O: ByteOrder> core::ops::$trait<$name<O>> for $native {
            type Output = $name<O>;

            #[inline(always)]
            fn $method(self, rhs: $name<O>) -> $name<O> {
                let rhs_native: $native = rhs.get();
                let result_native = core::ops::$trait::$method(self, rhs_native);
                $name::<O>::new(result_native)
            }
        }

        impl<O: ByteOrder> core::ops::$trait<$native> for $name<O> {
            type Output = $name<O>;

            #[inline(always)]
            fn $method(self, rhs: $native) -> $name<O> {
                let self_native: $native = self.get();
                let result_native = core::ops::$trait::$method(self_native, rhs);
                $name::<O>::new(result_native)
            }
        }

        impl<O: ByteOrder> core::ops::$trait_assign<$name<O>> for $name<O> {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: $name<O>) {
                *self = core::ops::$trait::$method(*self, rhs);
            }
        }

        impl<O: ByteOrder> core::ops::$trait_assign<$name<O>> for $native {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: $name<O>) {
                let rhs_native: $native = rhs.get();
                *self = core::ops::$trait::$method(*self, rhs_native);
            }
        }

        impl<O: ByteOrder> core::ops::$trait_assign<$native> for $name<O> {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: $native) {
                *self = core::ops::$trait::$method(*self, rhs);
            }
        }
    };
    // Implement traits in terms of the same trait on the native type, but
    // without performing a byte order swap when both operands are byteorder
    // types. This only works for bitwise operations like `&`, `|`, etc.
    //
    // When only one operand is a byteorder type, we still need to perform a
    // byteorder swap.
    (@without_byteorder_swap $name:ident, $native:ident, $trait:ident, $method:ident, $trait_assign:ident, $method_assign:ident) => {
        impl<O: ByteOrder> core::ops::$trait<$name<O>> for $name<O> {
            type Output = $name<O>;

            #[inline(always)]
            fn $method(self, rhs: $name<O>) -> $name<O> {
                let self_native = $native::from_ne_bytes(self.0);
                let rhs_native = $native::from_ne_bytes(rhs.0);
                let result_native = core::ops::$trait::$method(self_native, rhs_native);
                $name(result_native.to_ne_bytes(), PhantomData)
            }
        }

        impl<O: ByteOrder> core::ops::$trait<$name<O>> for $native {
            type Output = $name<O>;

            #[inline(always)]
            fn $method(self, rhs: $name<O>) -> $name<O> {
                // No runtime cost - just byte packing
                let rhs_native = $native::from_ne_bytes(rhs.0);
                // (Maybe) runtime cost - byte order swap
                let slf_byteorder = $name::<O>::new(self);
                // No runtime cost - just byte packing
                let slf_native = $native::from_ne_bytes(slf_byteorder.0);
                // Runtime cost - perform the operation
                let result_native = core::ops::$trait::$method(slf_native, rhs_native);
                // No runtime cost - just byte unpacking
                $name(result_native.to_ne_bytes(), PhantomData)
            }
        }

        impl<O: ByteOrder> core::ops::$trait<$native> for $name<O> {
            type Output = $name<O>;

            #[inline(always)]
            fn $method(self, rhs: $native) -> $name<O> {
                // (Maybe) runtime cost - byte order swap
                let rhs_byteorder = $name::<O>::new(rhs);
                // No runtime cost - just byte packing
                let rhs_native = $native::from_ne_bytes(rhs_byteorder.0);
                // No runtime cost - just byte packing
                let slf_native = $native::from_ne_bytes(self.0);
                // Runtime cost - perform the operation
                let result_native = core::ops::$trait::$method(slf_native, rhs_native);
                // No runtime cost - just byte unpacking
                $name(result_native.to_ne_bytes(), PhantomData)
            }
        }

        impl<O: ByteOrder> core::ops::$trait_assign<$name<O>> for $name<O> {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: $name<O>) {
                *self = core::ops::$trait::$method(*self, rhs);
            }
        }

        impl<O: ByteOrder> core::ops::$trait_assign<$name<O>> for $native {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: $name<O>) {
                // (Maybe) runtime cost - byte order swap
                let rhs_native = rhs.get();
                // Runtime cost - perform the operation
                *self = core::ops::$trait::$method(*self, rhs_native);
            }
        }

        impl<O: ByteOrder> core::ops::$trait_assign<$native> for $name<O> {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: $native) {
                *self = core::ops::$trait::$method(*self, rhs);
            }
        }
    };
}

macro_rules! doc_comment {
    ($x:expr, $($tt:tt)*) => {
        #[doc = $x]
        $($tt)*
    };
}

macro_rules! define_max_value_constant {
    ($name:ident, $bytes:expr, "unsigned integer") => {
        /// The maximum value.
        ///
        /// This constant should be preferred to constructing a new value using
        /// `new`, as `new` may perform an endianness swap depending on the
        /// endianness `O` and the endianness of the platform.
        pub const MAX_VALUE: $name<O> = $name([0xFFu8; $bytes], PhantomData);
    };
    // We don't provide maximum and minimum value constants for signed values
    // and floats because there's no way to do it generically - it would require
    // a different value depending on the value of the `ByteOrder` type
    // parameter. Currently, one workaround would be to provide implementations
    // for concrete implementations of that trait. In the long term, if we are
    // ever able to make the `new` constructor a const fn, we could use that
    // instead.
    ($name:ident, $bytes:expr, "signed integer") => {};
    ($name:ident, $bytes:expr, "floating point number") => {};
}

macro_rules! define_type {
    (
        $article:ident,
        $description:expr,
        $name:ident,
        $native:ident,
        $bits:expr,
        $bytes:expr,
        $from_be_fn:path,
        $to_be_fn:path,
        $from_le_fn:path,
        $to_le_fn:path,
        $number_kind:tt,
        [$($larger_native:ty),*],
        [$($larger_native_try:ty),*],
        [$($larger_byteorder:ident),*],
        [$($larger_byteorder_try:ident),*]
    ) => {
        doc_comment! {
            concat!($description, " stored in a given byte order.

`", stringify!($name), "` is like the native `", stringify!($native), "` type with
two major differences: First, it has no alignment requirement (its alignment is 1).
Second, the endianness of its memory layout is given by the type parameter `O`,
which can be any type which implements [`ByteOrder`]. In particular, this refers
to [`BigEndian`], [`LittleEndian`], [`NativeEndian`], and [`NetworkEndian`].

", stringify!($article), " `", stringify!($name), "` can be constructed using
the [`new`] method, and its contained value can be obtained as a native
`",stringify!($native), "` using the [`get`] method, or updated in place with
the [`set`] method. In all cases, if the endianness `O` is not the same as the
endianness of the current platform, an endianness swap will be performed in
order to uphold the invariants that a) the layout of `", stringify!($name), "`
has endianness `O` and that, b) the layout of `", stringify!($native), "` has
the platform's native endianness.

`", stringify!($name), "` implements [`FromBytes`], [`IntoBytes`], and [`Unaligned`],
making it useful for parsing and serialization. See the module documentation for an
example of how it can be used for parsing UDP packets.

[`new`]: crate::byteorder::", stringify!($name), "::new
[`get`]: crate::byteorder::", stringify!($name), "::get
[`set`]: crate::byteorder::", stringify!($name), "::set
[`FromBytes`]: crate::FromBytes
[`IntoBytes`]: crate::IntoBytes
[`Unaligned`]: crate::Unaligned"),
            #[derive(Copy, Clone, Eq, PartialEq, Hash)]
            #[cfg_attr(any(test),derive (KnownLayout , Immutable , FromBytes , IntoBytes , Unaligned))]
            #[repr(transparent)]
            pub struct $name<O>([u8; $bytes], PhantomData<O>);
        }

        #[cfg(not(any(test)))]
        impl_known_layout!(O => $name<O>);

        #[allow(unused_unsafe)] // Unused when `feature = "derive"`.
        // SAFETY: `$name<O>` is `repr(transparent)`, and so it has the same
        // layout as its only non-zero field, which is a `u8` array. `u8` arrays
        // are `Immutable`, `TryFromBytes`, `FromZeros`, `FromBytes`,
        // `IntoBytes`, and `Unaligned`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            impl_or_verify!(O => Immutable for $name<O>);
            impl_or_verify!(O => TryFromBytes for $name<O>);
            impl_or_verify!(O => FromZeros for $name<O>);
            impl_or_verify!(O => FromBytes for $name<O>);
            impl_or_verify!(O => IntoBytes for $name<O>);
            impl_or_verify!(O => Unaligned for $name<O>);
        };

        impl<O> Default for $name<O> {
            #[inline(always)]
            fn default() -> $name<O> {
                $name::ZERO
            }
        }

        impl<O> $name<O> {
            /// The value zero.
            ///
            /// This constant should be preferred to constructing a new value
            /// using `new`, as `new` may perform an endianness swap depending
            /// on the endianness and platform.
            pub const ZERO: $name<O> = $name([0u8; $bytes], PhantomData);

            define_max_value_constant!($name, $bytes, $number_kind);

            /// Constructs a new value from bytes which are already in `O` byte
            /// order.
            #[must_use = "has no side effects"]
            #[inline(always)]
            pub const fn from_bytes(bytes: [u8; $bytes]) -> $name<O> {
                $name(bytes, PhantomData)
            }

            /// Extracts the bytes of `self` without swapping the byte order.
            ///
            /// The returned bytes will be in `O` byte order.
            #[must_use = "has no side effects"]
            #[inline(always)]
            pub const fn to_bytes(self) -> [u8; $bytes] {
                self.0
            }
        }

        impl<O: ByteOrder> $name<O> {
            maybe_const_trait_bounded_fn! {
                /// Constructs a new value, possibly performing an endianness
                /// swap to guarantee that the returned value has endianness
                /// `O`.
                #[must_use = "has no side effects"]
                #[inline(always)]
                pub const fn new(n: $native) -> $name<O> {
                    let bytes = match O::ORDER {
                        Order::BigEndian => $to_be_fn(n),
                        Order::LittleEndian => $to_le_fn(n),
                    };

                    $name(bytes, PhantomData)
                }
            }

            maybe_const_trait_bounded_fn! {
                /// Returns the value as a primitive type, possibly performing
                /// an endianness swap to guarantee that the return value has
                /// the endianness of the native platform.
                #[must_use = "has no side effects"]
                #[inline(always)]
                pub const fn get(self) -> $native {
                    match O::ORDER {
                        Order::BigEndian => $from_be_fn(self.0),
                        Order::LittleEndian => $from_le_fn(self.0),
                    }
                }
            }

            /// Updates the value in place as a primitive type, possibly
            /// performing an endianness swap to guarantee that the stored value
            /// has the endianness `O`.
            #[inline(always)]
            pub fn set(&mut self, n: $native) {
                *self = Self::new(n);
            }
        }

        // The reasoning behind which traits to implement here is to only
        // implement traits which won't cause inference issues. Notably,
        // comparison traits like PartialEq and PartialOrd tend to cause
        // inference issues.

        impl<O: ByteOrder> From<$name<O>> for [u8; $bytes] {
            #[inline(always)]
            fn from(x: $name<O>) -> [u8; $bytes] {
                x.0
            }
        }

        impl<O: ByteOrder> From<[u8; $bytes]> for $name<O> {
            #[inline(always)]
            fn from(bytes: [u8; $bytes]) -> $name<O> {
                $name(bytes, PhantomData)
            }
        }

        impl<O: ByteOrder> From<$name<O>> for $native {
            #[inline(always)]
            fn from(x: $name<O>) -> $native {
                x.get()
            }
        }

        impl<O: ByteOrder> From<$native> for $name<O> {
            #[inline(always)]
            fn from(x: $native) -> $name<O> {
                $name::new(x)
            }
        }

        $(
            impl<O: ByteOrder> From<$name<O>> for $larger_native {
                #[inline(always)]
                fn from(x: $name<O>) -> $larger_native {
                    x.get().into()
                }
            }
        )*

        $(
            impl<O: ByteOrder> TryFrom<$larger_native_try> for $name<O> {
                type Error = TryFromIntError;
                #[inline(always)]
                fn try_from(x: $larger_native_try) -> Result<$name<O>, TryFromIntError> {
                    $native::try_from(x).map($name::new)
                }
            }
        )*

        $(
            impl<O: ByteOrder, P: ByteOrder> From<$name<O>> for $larger_byteorder<P> {
                #[inline(always)]
                fn from(x: $name<O>) -> $larger_byteorder<P> {
                    $larger_byteorder::new(x.get().into())
                }
            }
        )*

        $(
            impl<O: ByteOrder, P: ByteOrder> TryFrom<$larger_byteorder_try<P>> for $name<O> {
                type Error = TryFromIntError;
                #[inline(always)]
                fn try_from(x: $larger_byteorder_try<P>) -> Result<$name<O>, TryFromIntError> {
                    x.get().try_into().map($name::new)
                }
            }
        )*

        impl<O> AsRef<[u8; $bytes]> for $name<O> {
            #[inline(always)]
            fn as_ref(&self) -> &[u8; $bytes] {
                &self.0
            }
        }

        impl<O> AsMut<[u8; $bytes]> for $name<O> {
            #[inline(always)]
            fn as_mut(&mut self) -> &mut [u8; $bytes] {
                &mut self.0
            }
        }

        impl<O> PartialEq<$name<O>> for [u8; $bytes] {
            #[inline(always)]
            fn eq(&self, other: &$name<O>) -> bool {
                self.eq(&other.0)
            }
        }

        impl<O> PartialEq<[u8; $bytes]> for $name<O> {
            #[inline(always)]
            fn eq(&self, other: &[u8; $bytes]) -> bool {
                self.0.eq(other)
            }
        }

        impl<O: ByteOrder> PartialEq<$native> for $name<O> {
            #[inline(always)]
            fn eq(&self, other: &$native) -> bool {
                self.get().eq(other)
            }
        }

        impl_dbg_traits!($name, $native, $number_kind);
        impl_fmt_traits!($name, $native, $number_kind);
        impl_ops_traits!($name, $native, $number_kind);
    };
}

define_type!(
    A,
    "A 16-bit unsigned integer",
    U16,
    u16,
    16,
    2,
    u16::from_be_bytes,
    u16::to_be_bytes,
    u16::from_le_bytes,
    u16::to_le_bytes,
    "unsigned integer",
    [u32, u64, u128, usize],
    [u32, u64, u128, usize],
    [U32, U64, U128, Usize],
    [U32, U64, U128, Usize]
);
define_type!(
    A,
    "A 32-bit unsigned integer",
    U32,
    u32,
    32,
    4,
    u32::from_be_bytes,
    u32::to_be_bytes,
    u32::from_le_bytes,
    u32::to_le_bytes,
    "unsigned integer",
    [u64, u128],
    [u64, u128],
    [U64, U128],
    [U64, U128]
);
define_type!(
    A,
    "A 64-bit unsigned integer",
    U64,
    u64,
    64,
    8,
    u64::from_be_bytes,
    u64::to_be_bytes,
    u64::from_le_bytes,
    u64::to_le_bytes,
    "unsigned integer",
    [u128],
    [u128],
    [U128],
    [U128]
);
define_type!(
    A,
    "A 128-bit unsigned integer",
    U128,
    u128,
    128,
    16,
    u128::from_be_bytes,
    u128::to_be_bytes,
    u128::from_le_bytes,
    u128::to_le_bytes,
    "unsigned integer",
    [],
    [],
    [],
    []
);
define_type!(
    A,
    "A word-sized unsigned integer",
    Usize,
    usize,
    mem::size_of::<usize>() * 8,
    mem::size_of::<usize>(),
    usize::from_be_bytes,
    usize::to_be_bytes,
    usize::from_le_bytes,
    usize::to_le_bytes,
    "unsigned integer",
    [],
    [],
    [],
    []
);
define_type!(
    An,
    "A 16-bit signed integer",
    I16,
    i16,
    16,
    2,
    i16::from_be_bytes,
    i16::to_be_bytes,
    i16::from_le_bytes,
    i16::to_le_bytes,
    "signed integer",
    [i32, i64, i128, isize],
    [i32, i64, i128, isize],
    [I32, I64, I128, Isize],
    [I32, I64, I128, Isize]
);
define_type!(
    An,
    "A 32-bit signed integer",
    I32,
    i32,
    32,
    4,
    i32::from_be_bytes,
    i32::to_be_bytes,
    i32::from_le_bytes,
    i32::to_le_bytes,
    "signed integer",
    [i64, i128],
    [i64, i128],
    [I64, I128],
    [I64, I128]
);
define_type!(
    An,
    "A 64-bit signed integer",
    I64,
    i64,
    64,
    8,
    i64::from_be_bytes,
    i64::to_be_bytes,
    i64::from_le_bytes,
    i64::to_le_bytes,
    "signed integer",
    [i128],
    [i128],
    [I128],
    [I128]
);
define_type!(
    An,
    "A 128-bit signed integer",
    I128,
    i128,
    128,
    16,
    i128::from_be_bytes,
    i128::to_be_bytes,
    i128::from_le_bytes,
    i128::to_le_bytes,
    "signed integer",
    [],
    [],
    [],
    []
);
define_type!(
    An,
    "A word-sized signed integer",
    Isize,
    isize,
    mem::size_of::<isize>() * 8,
    mem::size_of::<isize>(),
    isize::from_be_bytes,
    isize::to_be_bytes,
    isize::from_le_bytes,
    isize::to_le_bytes,
    "signed integer",
    [],
    [],
    [],
    []
);

// FIXME(https://github.com/rust-lang/rust/issues/72447): Use the endianness
// conversion methods directly once those are const-stable.
macro_rules! define_float_conversion {
    ($ty:ty, $bits:ident, $bytes:expr, $mod:ident) => {
        mod $mod {
            use super::*;

            define_float_conversion!($ty, $bits, $bytes, from_be_bytes, to_be_bytes);
            define_float_conversion!($ty, $bits, $bytes, from_le_bytes, to_le_bytes);
        }
    };
    ($ty:ty, $bits:ident, $bytes:expr, $from:ident, $to:ident) => {
        // Clippy: The suggestion of using `from_bits()` instead doesn't work
        // because `from_bits` is not const-stable on our MSRV.
        #[allow(clippy::unnecessary_transmutes)]
        pub(crate) const fn $from(bytes: [u8; $bytes]) -> $ty {
            transmute!($bits::$from(bytes))
        }

        pub(crate) const fn $to(f: $ty) -> [u8; $bytes] {
            // Clippy: The suggestion of using `f.to_bits()` instead doesn't
            // work because `to_bits` is not const-stable on our MSRV.
            #[allow(clippy::unnecessary_transmutes)]
            let bits: $bits = transmute!(f);
            bits.$to()
        }
    };
}

define_float_conversion!(f32, u32, 4, f32_ext);
define_float_conversion!(f64, u64, 8, f64_ext);

define_type!(
    An,
    "A 32-bit floating point number",
    F32,
    f32,
    32,
    4,
    f32_ext::from_be_bytes,
    f32_ext::to_be_bytes,
    f32_ext::from_le_bytes,
    f32_ext::to_le_bytes,
    "floating point number",
    [f64],
    [],
    [F64],
    []
);
define_type!(
    An,
    "A 64-bit floating point number",
    F64,
    f64,
    64,
    8,
    f64_ext::from_be_bytes,
    f64_ext::to_be_bytes,
    f64_ext::from_le_bytes,
    f64_ext::to_le_bytes,
    "floating point number",
    [],
    [],
    [],
    []
);

macro_rules! module {
    ($name:ident, $trait:ident, $endianness_str:expr) => {
        /// Numeric primitives stored in
        #[doc = $endianness_str]
        /// byte order.
        pub mod $name {
            use super::$trait;

            module!(@ty U16,  $trait, "16-bit unsigned integer", $endianness_str);
            module!(@ty U32,  $trait, "32-bit unsigned integer", $endianness_str);
            module!(@ty U64,  $trait, "64-bit unsigned integer", $endianness_str);
            module!(@ty U128, $trait, "128-bit unsigned integer", $endianness_str);
            module!(@ty I16,  $trait, "16-bit signed integer", $endianness_str);
            module!(@ty I32,  $trait, "32-bit signed integer", $endianness_str);
            module!(@ty I64,  $trait, "64-bit signed integer", $endianness_str);
            module!(@ty I128, $trait, "128-bit signed integer", $endianness_str);
            module!(@ty F32,  $trait, "32-bit floating point number", $endianness_str);
            module!(@ty F64,  $trait, "64-bit floating point number", $endianness_str);
        }
    };
    (@ty $ty:ident, $trait:ident, $desc_str:expr, $endianness_str:expr) => {
        /// A
        #[doc = $desc_str]
        /// stored in
        #[doc = $endianness_str]
        /// byte order.
        pub type $ty = crate::byteorder::$ty<$trait>;
    };
}

module!(big_endian, BigEndian, "big-endian");
module!(little_endian, LittleEndian, "little-endian");
module!(network_endian, NetworkEndian, "network-endian");
module!(native_endian, NativeEndian, "native-endian");

#[cfg(any(test, kani))]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/unit/src/byteorder/guarded_tests_1009.rs"));
