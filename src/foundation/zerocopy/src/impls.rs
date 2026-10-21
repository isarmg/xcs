// SPDX-License-Identifier: BSD-2-Clause OR Apache-2.0 OR MIT
//
// Copyright 2024 The Fuchsia Authors
//
// Licensed under the 2-Clause BSD License <LICENSE-BSD or
// https://opensource.org/license/bsd-2-clause>, Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0>, or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed except according to
// those terms.

use core::{
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit as CoreMaybeUninit,
    ptr::NonNull,
};

use super::*;
use crate::pointer::cast::{CastSizedExact, CastUnsized};

// SAFETY: Per the reference [1], "the unit tuple (`()`) ... is guaranteed as a
// zero-sized type to have a size of 0 and an alignment of 1."
// - `Immutable`: `()` self-evidently does not contain any `UnsafeCell`s.
// - `TryFromBytes` (with no validator), `FromZeros`, `FromBytes`: There is only
//   one possible sequence of 0 bytes, and `()` is inhabited.
// - `IntoBytes`: Since `()` has size 0, it contains no padding bytes.
// - `Unaligned`: `()` has alignment 1.
//
// [1] https://doc.rust-lang.org/1.81.0/reference/type-layout.html#tuple-layout
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!((): Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
    assert_unaligned!(());
};

// SAFETY:
// - `Immutable`: These types self-evidently do not contain any `UnsafeCell`s.
// - `TryFromBytes` (with no validator), `FromZeros`, `FromBytes`: all bit
//   patterns are valid for numeric types [1]
// - `IntoBytes`: numeric types have no padding bytes [1]
// - `Unaligned` (`u8` and `i8` only): The reference [2] specifies the size of
//   `u8` and `i8` as 1 byte. We also know that:
//   - Alignment is >= 1 [3]
//   - Size is an integer multiple of alignment [4]
//   - The only value >= 1 for which 1 is an integer multiple is 1 Therefore,
//   the only possible alignment for `u8` and `i8` is 1.
//
// [1] Per https://doc.rust-lang.org/1.81.0/reference/types/numeric.html#bit-validity:
//
//     For every numeric type, `T`, the bit validity of `T` is equivalent to
//     the bit validity of `[u8; size_of::<T>()]`. An uninitialized byte is
//     not a valid `u8`.
//
// [2] https://doc.rust-lang.org/1.81.0/reference/type-layout.html#primitive-data-layout
//
// [3] Per https://doc.rust-lang.org/1.81.0/reference/type-layout.html#size-and-alignment:
//
//     Alignment is measured in bytes, and must be at least 1.
//
// [4] Per https://doc.rust-lang.org/1.81.0/reference/type-layout.html#size-and-alignment:
//
//     The size of a value is always a multiple of its alignment.
//
// FIXME(#278): Once we've updated the trait docs to refer to `u8`s rather than
// bits or bytes, update this comment, especially the reference to [1].
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(u8: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
    unsafe_impl!(i8: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
    assert_unaligned!(u8, i8);
    unsafe_impl!(u16: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(i16: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(u32: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(i32: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(u64: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(i64: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(u128: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(i128: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(usize: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(isize: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(f32: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(f64: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    #[cfg(feature = "float-nightly")]
    unsafe_impl!(#[cfg_attr(doc_cfg, doc(cfg(feature = "float-nightly")))] f16: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
    #[cfg(feature = "float-nightly")]
    unsafe_impl!(#[cfg_attr(doc_cfg, doc(cfg(feature = "float-nightly")))] f128: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes);
};

// SAFETY:
// - `Immutable`: `bool` self-evidently does not contain any `UnsafeCell`s.
// - `FromZeros`: Valid since "[t]he value false has the bit pattern 0x00" [1].
// - `IntoBytes`: Since "the boolean type has a size and alignment of 1 each"
//   and "The value false has the bit pattern 0x00 and the value true has the
//   bit pattern 0x01" [1]. Thus, the only byte of the bool is always
//   initialized.
// - `Unaligned`: Per the reference [1], "[a]n object with the boolean type has
//   a size and alignment of 1 each."
//
// [1] https://doc.rust-lang.org/1.81.0/reference/types/boolean.html
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe { unsafe_impl!(bool: Immutable, FromZeros, IntoBytes, Unaligned) };
assert_unaligned!(bool);

// SAFETY: The impl must only return `true` for its argument if the original
// `Maybe<bool>` refers to a valid `bool`. We only return true if the `u8` value
// is 0 or 1, and both of these are valid values for `bool` [1].
//
// [1] Per https://doc.rust-lang.org/1.81.0/reference/types/boolean.html:
//
//   The value false has the bit pattern 0x00 and the value true has the bit
//   pattern 0x01.
const _: () = unsafe {
    unsafe_impl!(=> TryFromBytes for bool; |byte| {
        let byte = byte.transmute_with::<u8, invariant::Safe, CastSizedExact, BecauseImmutable>();
        *byte.unaligned_as_ref() < 2
    })
};

// SAFETY:
// - `Immutable`: `char` self-evidently does not contain any `UnsafeCell`s.
// - `FromZeros`: Per reference [1], "[a] value of type char is a Unicode scalar
//   value (i.e. a code point that is not a surrogate), represented as a 32-bit
//   unsigned word in the 0x0000 to 0xD7FF or 0xE000 to 0x10FFFF range" which
//   contains 0x0000.
// - `IntoBytes`: `char` is per reference [1] "represented as a 32-bit unsigned
//   word" (`u32`) which is `IntoBytes`. Note that unlike `u32`, not all bit
//   patterns are valid for `char`.
//
// [1] https://doc.rust-lang.org/1.81.0/reference/types/textual.html
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe { unsafe_impl!(char: Immutable, FromZeros, IntoBytes) };

// SAFETY: The impl must only return `true` for its argument if the original
// `Maybe<char>` refers to a valid `char`. `char::from_u32` guarantees that it
// returns `None` if its input is not a valid `char` [1].
//
// [1] Per https://doc.rust-lang.org/core/primitive.char.html#method.from_u32:
//
//   `from_u32()` will return `None` if the input is not a valid value for a
//   `char`.
const _: () = unsafe {
    unsafe_impl!(=> TryFromBytes for char; |c| {
        let c = c.transmute_with::<Unalign<u32>, invariant::Safe, CastSizedExact, BecauseImmutable>();
        let c = c.read().into_inner();
        char::from_u32(c).is_some()
    });
};

// SAFETY: Per the Reference [1], `str` has the same layout as `[u8]`.
// - `Immutable`: `[u8]` does not contain any `UnsafeCell`s.
// - `FromZeros`, `IntoBytes`, `Unaligned`: `[u8]` is `FromZeros`, `IntoBytes`,
//   and `Unaligned`.
//
// Note that we don't `assert_unaligned!(str)` because `assert_unaligned!` uses
// `align_of`, which only works for `Sized` types.
//
// FIXME(#429): Improve safety proof for `FromZeros` and `IntoBytes`; having the same
// layout as `[u8]` isn't sufficient.
//
// [1] Per https://doc.rust-lang.org/1.81.0/reference/type-layout.html#str-layout:
//
//   String slices are a UTF-8 representation of characters that have the same
//   layout as slices of type `[u8]`.
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe { unsafe_impl!(str: Immutable, FromZeros, IntoBytes, Unaligned) };

// SAFETY: The impl must only return `true` for its argument if the original
// `Maybe<str>` refers to a valid `str`. `str::from_utf8` guarantees that it
// returns `Err` if its input is not a valid `str` [1].
//
// [1] Per https://doc.rust-lang.org/core/str/fn.from_utf8.html#errors:
//
//   Returns `Err` if the slice is not UTF-8.
const _: () = unsafe {
    unsafe_impl!(=> TryFromBytes for str; |c| {
        let c = c.transmute_with::<[u8], invariant::Safe, CastUnsized, BecauseImmutable>();
        let c = c.unaligned_as_ref();
        core::str::from_utf8(c).is_ok()
    })
};

macro_rules! unsafe_impl_try_from_bytes_for_nonzero {
    ($($nonzero:ident[$prim:ty]),*) => {
        $(
            unsafe_impl!(=> TryFromBytes for $nonzero; |n| {
                let n = n.transmute_with::<Unalign<$prim>, invariant::Safe, CastSizedExact, BecauseImmutable>();
                $nonzero::new(n.read().into_inner()).is_some()
            });
        )*
    }
}

// `NonZeroXxx` is `IntoBytes`, but not `FromZeros` or `FromBytes`.
//
// SAFETY:
// - `IntoBytes`: `NonZeroXxx` has the same layout as its associated primitive.
//    Since it is the same size, this guarantees it has no padding - integers
//    have no padding, and there's no room for padding if it can represent all
//    of the same values except 0.
// - `Unaligned`: `NonZeroU8` and `NonZeroI8` document that `Option<NonZeroU8>`
//   and `Option<NonZeroI8>` both have size 1. [1] [2] This is worded in a way
//   that makes it unclear whether it's meant as a guarantee, but given the
//   purpose of those types, it's virtually unthinkable that that would ever
//   change. `Option` cannot be smaller than its contained type, which implies
//   that, and `NonZeroX8` are of size 1 or 0. `NonZeroX8` can represent
//   multiple states, so they cannot be 0 bytes, which means that they must be 1
//   byte. The only valid alignment for a 1-byte type is 1.
//
// FIXME(#429):
// - Add quotes from documentation.
// - Add safety comment for `Immutable`. How can we prove that `NonZeroXxx`
//   doesn't contain any `UnsafeCell`s? It's obviously true, but it's not clear
//   how we'd prove it short of adding text to the stdlib docs that says so
//   explicitly, which likely wouldn't be accepted.
//
// [1] Per https://doc.rust-lang.org/1.81.0/std/num/type.NonZeroU8.html:
//
//     `NonZeroU8` is guaranteed to have the same layout and bit validity as `u8` with
//     the exception that 0 is not a valid instance.
//
// [2] Per https://doc.rust-lang.org/1.81.0/std/num/type.NonZeroI8.html:
//
//     `NonZeroI8` is guaranteed to have the same layout and bit validity as `i8` with
//     the exception that 0 is not a valid instance.
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(NonZeroU8: Immutable, IntoBytes, Unaligned);
    unsafe_impl!(NonZeroI8: Immutable, IntoBytes, Unaligned);
    assert_unaligned!(NonZeroU8, NonZeroI8);
    unsafe_impl!(NonZeroU16: Immutable, IntoBytes);
    unsafe_impl!(NonZeroI16: Immutable, IntoBytes);
    unsafe_impl!(NonZeroU32: Immutable, IntoBytes);
    unsafe_impl!(NonZeroI32: Immutable, IntoBytes);
    unsafe_impl!(NonZeroU64: Immutable, IntoBytes);
    unsafe_impl!(NonZeroI64: Immutable, IntoBytes);
    unsafe_impl!(NonZeroU128: Immutable, IntoBytes);
    unsafe_impl!(NonZeroI128: Immutable, IntoBytes);
    unsafe_impl!(NonZeroUsize: Immutable, IntoBytes);
    unsafe_impl!(NonZeroIsize: Immutable, IntoBytes);
    unsafe_impl_try_from_bytes_for_nonzero!(
        NonZeroU8[u8],
        NonZeroI8[i8],
        NonZeroU16[u16],
        NonZeroI16[i16],
        NonZeroU32[u32],
        NonZeroI32[i32],
        NonZeroU64[u64],
        NonZeroI64[i64],
        NonZeroU128[u128],
        NonZeroI128[i128],
        NonZeroUsize[usize],
        NonZeroIsize[isize]
    );
};

// SAFETY:
// - `TryFromBytes` (with no validator), `FromZeros`, `FromBytes`, `IntoBytes`:
//   The Rust compiler reuses `0` value to represent `None`, so
//   `size_of::<Option<NonZeroXxx>>() == size_of::<xxx>()`; see `NonZeroXxx`
//   documentation.
// - `Unaligned`: `NonZeroU8` and `NonZeroI8` document that `Option<NonZeroU8>`
//   and `Option<NonZeroI8>` both have size 1. [1] [2] This is worded in a way
//   that makes it unclear whether it's meant as a guarantee, but given the
//   purpose of those types, it's virtually unthinkable that that would ever
//   change. The only valid alignment for a 1-byte type is 1.
//
// [1] Per https://doc.rust-lang.org/1.81.0/std/num/type.NonZeroU8.html:
//
//     `Option<NonZeroU8>` is guaranteed to be compatible with `u8`, including in FFI.
//
//     Thanks to the null pointer optimization, `NonZeroU8` and `Option<NonZeroU8>`
//     are guaranteed to have the same size and alignment:
//
// [2] Per https://doc.rust-lang.org/1.81.0/std/num/type.NonZeroI8.html:
//
//     `Option<NonZeroI8>` is guaranteed to be compatible with `i8`, including in FFI.
//
//     Thanks to the null pointer optimization, `NonZeroI8` and `Option<NonZeroI8>`
//     are guaranteed to have the same size and alignment:
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(Option<NonZeroU8>: TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
    unsafe_impl!(Option<NonZeroI8>: TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
    assert_unaligned!(Option<NonZeroU8>, Option<NonZeroI8>);
    unsafe_impl!(Option<NonZeroU16>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroI16>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroU32>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroI32>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroU64>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroI64>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroU128>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroI128>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroUsize>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
    unsafe_impl!(Option<NonZeroIsize>: TryFromBytes, FromZeros, FromBytes, IntoBytes);
};

// SAFETY: While it's not fully documented, the consensus is that `Box<T>` does
// not contain any `UnsafeCell`s for `T: Sized` [1]. This is not a complete
// proof, but we are accepting this as a known risk per #1358.
//
// [1] https://github.com/rust-lang/unsafe-code-guidelines/issues/492
#[cfg(feature = "alloc")]
const _: () = unsafe {
    unsafe_impl!(
        #[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
        T: Sized => Immutable for Box<T>
    )
};

// SAFETY: The following types can be transmuted from `[0u8; size_of::<T>()]`. [1]
//
// [1] Per https://doc.rust-lang.org/1.89.0/core/option/index.html#representation:
//
//   Rust guarantees to optimize the following types `T` such that [`Option<T>`]
//   has the same size and alignment as `T`. In some of these cases, Rust
//   further guarantees that `transmute::<_, Option<T>>([0u8; size_of::<T>()])`
//   is sound and produces `Option::<T>::None`. These cases are identified by
//   the second column:
//
//   | `T`                               | `transmute::<_, Option<T>>([0u8; size_of::<T>()])` sound? |
//   |-----------------------------------|-----------------------------------------------------------|
//   | [`Box<U>`]                        | when `U: Sized`                                           |
//   | `&U`                              | when `U: Sized`                                           |
//   | `&mut U`                          | when `U: Sized`                                           |
//   | [`ptr::NonNull<U>`]               | when `U: Sized`                                           |
//   | `fn`, `extern "C" fn`[^extern_fn] | always                                                    |
//
//   [^extern_fn]: this remains true for `unsafe` variants, any argument/return
//     types, and any other ABI: `[unsafe] extern "abi" fn` (_e.g._, `extern
//     "system" fn`)
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    #[cfg(feature = "alloc")]
    unsafe_impl!(
        #[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
        T => TryFromBytes for Option<Box<T>>; |c| pointer::is_zeroed(c)
    );
    #[cfg(feature = "alloc")]
    unsafe_impl!(
        #[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
        T => FromZeros for Option<Box<T>>
    );
    unsafe_impl!(
        T => TryFromBytes for Option<&'_ T>; |c| pointer::is_zeroed(c)
    );
    unsafe_impl!(T => FromZeros for Option<&'_ T>);
    unsafe_impl!(
            T => TryFromBytes for Option<&'_ mut T>; |c| pointer::is_zeroed(c)
    );
    unsafe_impl!(T => FromZeros for Option<&'_ mut T>);
    unsafe_impl!(
        T => TryFromBytes for Option<NonNull<T>>; |c| pointer::is_zeroed(c)
    );
    unsafe_impl!(T => FromZeros for Option<NonNull<T>>);
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => FromZeros for opt_fn!(...));
    unsafe_impl_for_power_set!(
        A, B, C, D, E, F, G, H, I, J, K, L -> M => TryFromBytes for opt_fn!(...);
        |c| pointer::is_zeroed(c)
    );
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => FromZeros for opt_unsafe_fn!(...));
    unsafe_impl_for_power_set!(
        A, B, C, D, E, F, G, H, I, J, K, L -> M => TryFromBytes for opt_unsafe_fn!(...);
        |c| pointer::is_zeroed(c)
    );
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => FromZeros for opt_extern_c_fn!(...));
    unsafe_impl_for_power_set!(
        A, B, C, D, E, F, G, H, I, J, K, L -> M => TryFromBytes for opt_extern_c_fn!(...);
        |c| pointer::is_zeroed(c)
    );
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => FromZeros for opt_unsafe_extern_c_fn!(...));
    unsafe_impl_for_power_set!(
        A, B, C, D, E, F, G, H, I, J, K, L -> M => TryFromBytes for opt_unsafe_extern_c_fn!(...);
        |c| pointer::is_zeroed(c)
    );
};

// SAFETY: `[unsafe] [extern "C"] fn()` self-evidently do not contain
// `UnsafeCell`s. This is not a proof, but we are accepting this as a known risk
// per #1358.
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => Immutable for opt_fn!(...));
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => Immutable for opt_unsafe_fn!(...));
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => Immutable for opt_extern_c_fn!(...));
    unsafe_impl_for_power_set!(A, B, C, D, E, F, G, H, I, J, K, L -> M => Immutable for opt_unsafe_extern_c_fn!(...));
};

#[cfg(all(
    not(no_zerocopy_target_has_atomics_1_60_0),
    any(
        target_has_atomic = "8",
        target_has_atomic = "16",
        target_has_atomic = "32",
        target_has_atomic = "64",
        target_has_atomic = "ptr"
    )
))]

mod atomics {
    use super::*;

    macro_rules! impl_traits_for_atomics {
        ($($atomics:tt [$primitives:ty]),* $(,)?) => {
            $(
                impl_known_layout!($atomics);
                impl_for_transmute_from!(=> FromZeros for $atomics [$primitives]);
                impl_for_transmute_from!(=> FromBytes for $atomics [$primitives]);
                impl_for_transmute_from!(=> TryFromBytes for $atomics [$primitives]);
                impl_for_transmute_from!(=> IntoBytes for $atomics [$primitives]);
            )*
        };
    }

    /// Implements `TransmuteFrom` for `$atomic`, `$prim`, and
    /// `UnsafeCell<$prim>`.
    ///
    /// # Safety
    ///
    /// `$atomic` must have the same size and bit validity as `$prim`.
    macro_rules! unsafe_impl_transmute_from_for_atomic {
        ($($($tyvar:ident)? => $atomic:ty [$prim:ty]),*) => {{
            crate::util::macros::__unsafe();

            use crate::pointer::{SizeEq, TransmuteFrom, invariant::Safe};

            $(
                // SAFETY: The caller promised that `$atomic` and `$prim` have
                // the same size and bit validity.
                unsafe impl<$($tyvar)?> TransmuteFrom<$atomic, Safe, Safe> for $prim {}
                // SAFETY: The caller promised that `$atomic` and `$prim` have
                // the same size and bit validity.
                unsafe impl<$($tyvar)?> TransmuteFrom<$prim, Safe, Safe> for $atomic {}

                impl<$($tyvar)?> SizeEq<ReadOnly<$atomic>> for ReadOnly<$prim> {
                    type CastFrom = $crate::pointer::cast::CastSizedExact;
                }

                // SAFETY: The caller promised that `$atomic` and `$prim` have
                // the same bit validity. `UnsafeCell<T>` has the same bit
                // validity as `T` [1].
                //
                // [1] Per https://doc.rust-lang.org/1.85.0/std/cell/struct.UnsafeCell.html#memory-layout:
                //
                //   `UnsafeCell<T>` has the same in-memory representation as
                //   its inner type `T`. A consequence of this guarantee is that
                //   it is possible to convert between `T` and `UnsafeCell<T>`.
                unsafe impl<$($tyvar)?> TransmuteFrom<$atomic, Safe, Safe> for core::cell::UnsafeCell<$prim> {}
                // SAFETY: See previous safety comment.
                unsafe impl<$($tyvar)?> TransmuteFrom<core::cell::UnsafeCell<$prim>, Safe, Safe> for $atomic {}
            )*
        }};
    }

    #[cfg(target_has_atomic = "8")]

    mod atomic_8 {
        use core::sync::atomic::{AtomicBool, AtomicI8, AtomicU8};

        use super::*;

        impl_traits_for_atomics!(AtomicU8[u8], AtomicI8[i8]);

        impl_known_layout!(AtomicBool);
        impl_for_transmute_from!(=> FromZeros for AtomicBool [bool]);
        impl_for_transmute_from!(=> TryFromBytes for AtomicBool [bool]);
        impl_for_transmute_from!(=> IntoBytes for AtomicBool [bool]);

        // SAFETY: Per [1], `AtomicBool`, `AtomicU8`, and `AtomicI8` have the
        // same size as `bool`, `u8`, and `i8` respectively. Since a type's
        // alignment cannot be smaller than 1 [2], and since its alignment
        // cannot be greater than its size [3], the only possible value for the
        // alignment is 1. Thus, it is sound to implement `Unaligned`.
        //
        // [1] Per (for example) https://doc.rust-lang.org/1.81.0/std/sync/atomic/struct.AtomicU8.html:
        //
        //   This type has the same size, alignment, and bit validity as the
        //   underlying integer type
        //
        // [2] Per https://doc.rust-lang.org/1.81.0/reference/type-layout.html#size-and-alignment:
        //
        //     Alignment is measured in bytes, and must be at least 1.
        //
        // [3] Per https://doc.rust-lang.org/1.81.0/reference/type-layout.html#size-and-alignment:
        //
        //     The size of a value is always a multiple of its alignment.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            unsafe_impl!(AtomicBool: Unaligned);
            unsafe_impl!(AtomicU8: Unaligned);
            unsafe_impl!(AtomicI8: Unaligned);
            assert_unaligned!(AtomicBool, AtomicU8, AtomicI8);
        };

        // SAFETY: `AtomicU8`, `AtomicI8`, and `AtomicBool` have the same size
        // and bit validity as `u8`, `i8`, and `bool` respectively [1][2][3].
        //
        // [1] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicU8.html:
        //
        //   This type has the same size, alignment, and bit validity as the
        //   underlying integer type, `u8`.
        //
        // [2] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicI8.html:
        //
        //   This type has the same size, alignment, and bit validity as the
        //   underlying integer type, `i8`.
        //
        // [3] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicBool.html:
        //
        //   This type has the same size, alignment, and bit validity a `bool`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            unsafe_impl_transmute_from_for_atomic!(
                => AtomicU8 [u8],
                => AtomicI8 [i8],
                => AtomicBool [bool]
            )
        };
    }

    #[cfg(target_has_atomic = "16")]

    mod atomic_16 {
        use core::sync::atomic::{AtomicI16, AtomicU16};

        use super::*;

        impl_traits_for_atomics!(AtomicU16[u16], AtomicI16[i16]);

        // SAFETY: `AtomicU16` and `AtomicI16` have the same size and bit
        // validity as `u16` and `i16` respectively [1][2].
        //
        // [1] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicU16.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `u16`.
        //
        // [2] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicI16.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `i16`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            unsafe_impl_transmute_from_for_atomic!(=> AtomicU16 [u16], => AtomicI16 [i16])
        };
    }

    #[cfg(target_has_atomic = "32")]

    mod atomic_32 {
        use core::sync::atomic::{AtomicI32, AtomicU32};

        use super::*;

        impl_traits_for_atomics!(AtomicU32[u32], AtomicI32[i32]);

        // SAFETY: `AtomicU32` and `AtomicI32` have the same size and bit
        // validity as `u32` and `i32` respectively [1][2].
        //
        // [1] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicU32.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `u32`.
        //
        // [2] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicI32.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `i32`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            unsafe_impl_transmute_from_for_atomic!(=> AtomicU32 [u32], => AtomicI32 [i32])
        };
    }

    #[cfg(target_has_atomic = "64")]

    mod atomic_64 {
        use core::sync::atomic::{AtomicI64, AtomicU64};

        use super::*;

        impl_traits_for_atomics!(AtomicU64[u64], AtomicI64[i64]);

        // SAFETY: `AtomicU64` and `AtomicI64` have the same size and bit
        // validity as `u64` and `i64` respectively [1][2].
        //
        // [1] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicU64.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `u64`.
        //
        // [2] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicI64.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `i64`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            unsafe_impl_transmute_from_for_atomic!(=> AtomicU64 [u64], => AtomicI64 [i64])
        };
    }

    #[cfg(target_has_atomic = "ptr")]

    mod atomic_ptr {
        use core::sync::atomic::{AtomicIsize, AtomicPtr, AtomicUsize};

        use super::*;

        impl_traits_for_atomics!(AtomicUsize[usize], AtomicIsize[isize]);

        // FIXME(#170): Implement `FromBytes` and `IntoBytes` once we implement
        // those traits for `*mut T`.
        impl_known_layout!(T => AtomicPtr<T>);
        impl_for_transmute_from!(T => TryFromBytes for AtomicPtr<T> [*mut T]);
        impl_for_transmute_from!(T => FromZeros for AtomicPtr<T> [*mut T]);

        // SAFETY: `AtomicUsize` and `AtomicIsize` have the same size and bit
        // validity as `usize` and `isize` respectively [1][2].
        //
        // [1] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicUsize.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `usize`.
        //
        // [2] Per https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicIsize.html:
        //
        //   This type has the same size and bit validity as the underlying
        //   integer type, `isize`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe {
            unsafe_impl_transmute_from_for_atomic!(=> AtomicUsize [usize], => AtomicIsize [isize])
        };

        // SAFETY: Per
        // https://doc.rust-lang.org/1.85.0/std/sync/atomic/struct.AtomicPtr.html:
        //
        //   This type has the same size and bit validity as a `*mut T`.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        const _: () = unsafe { unsafe_impl_transmute_from_for_atomic!(T => AtomicPtr<T> [*mut T]) };
    }
}

// SAFETY: Per reference [1]: "For all T, the following are guaranteed:
// size_of::<PhantomData<T>>() == 0 align_of::<PhantomData<T>>() == 1". This
// gives:
// - `Immutable`: `PhantomData` has no fields.
// - `TryFromBytes` (with no validator), `FromZeros`, `FromBytes`: There is only
//   one possible sequence of 0 bytes, and `PhantomData` is inhabited.
// - `IntoBytes`: Since `PhantomData` has size 0, it contains no padding bytes.
// - `Unaligned`: Per the preceding reference, `PhantomData` has alignment 1.
//
// [1] https://doc.rust-lang.org/1.81.0/std/marker/struct.PhantomData.html#layout-1
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(T: ?Sized => Immutable for PhantomData<T>);
    unsafe_impl!(T: ?Sized => TryFromBytes for PhantomData<T>);
    unsafe_impl!(T: ?Sized => FromZeros for PhantomData<T>);
    unsafe_impl!(T: ?Sized => FromBytes for PhantomData<T>);
    unsafe_impl!(T: ?Sized => IntoBytes for PhantomData<T>);
    unsafe_impl!(T: ?Sized => Unaligned for PhantomData<T>);
    assert_unaligned!(PhantomData<()>, PhantomData<u8>, PhantomData<u64>);
};

impl_for_transmute_from!(T: TryFromBytes => TryFromBytes for Wrapping<T>[T]);
impl_for_transmute_from!(T: FromZeros => FromZeros for Wrapping<T>[T]);
impl_for_transmute_from!(T: FromBytes => FromBytes for Wrapping<T>[T]);
impl_for_transmute_from!(T: IntoBytes => IntoBytes for Wrapping<T>[T]);
assert_unaligned!(Wrapping<()>, Wrapping<u8>);

// SAFETY: Per [1], `Wrapping<T>` has the same layout as `T`. Since its single
// field (of type `T`) is public, it would be a breaking change to add or remove
// fields. Thus, we know that `Wrapping<T>` contains a `T` (as opposed to just
// having the same size and alignment as `T`) with no pre- or post-padding.
// Thus, `Wrapping<T>` must have `UnsafeCell`s covering the same byte ranges as
// `Inner = T`.
//
// [1] Per https://doc.rust-lang.org/1.81.0/std/num/struct.Wrapping.html#layout-1:
//
//   `Wrapping<T>` is guaranteed to have the same layout and ABI as `T`
const _: () = unsafe { unsafe_impl!(T: Immutable => Immutable for Wrapping<T>) };

// SAFETY: Per [1] in the preceding safety comment, `Wrapping<T>` has the same
// alignment as `T`.
const _: () = unsafe { unsafe_impl!(T: Unaligned => Unaligned for Wrapping<T>) };

// SAFETY: `TryFromBytes` (with no validator), `FromZeros`, `FromBytes`:
// `MaybeUninit<T>` has no restrictions on its contents.
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(T => TryFromBytes for CoreMaybeUninit<T>);
    unsafe_impl!(T => FromZeros for CoreMaybeUninit<T>);
    unsafe_impl!(T => FromBytes for CoreMaybeUninit<T>);
};

// SAFETY: `MaybeUninit<T>` has `UnsafeCell`s covering the same byte ranges as
// `Inner = T`. This is not explicitly documented, but it can be inferred. Per
// [1], `MaybeUninit<T>` has the same size as `T`. Further, note the signature
// of `MaybeUninit::assume_init_ref` [2]:
//
//   pub unsafe fn assume_init_ref(&self) -> &T
//
// If the argument `&MaybeUninit<T>` and the returned `&T` had `UnsafeCell`s at
// different offsets, this would be unsound. Its existence is proof that this is
// not the case.
//
// [1] Per https://doc.rust-lang.org/1.81.0/std/mem/union.MaybeUninit.html#layout-1:
//
// `MaybeUninit<T>` is guaranteed to have the same size, alignment, and ABI as
// `T`.
//
// [2] https://doc.rust-lang.org/1.81.0/std/mem/union.MaybeUninit.html#method.assume_init_ref
const _: () = unsafe { unsafe_impl!(T: Immutable => Immutable for CoreMaybeUninit<T>) };

// SAFETY: Per [1] in the preceding safety comment, `MaybeUninit<T>` has the
// same alignment as `T`.
const _: () = unsafe { unsafe_impl!(T: Unaligned => Unaligned for CoreMaybeUninit<T>) };
assert_unaligned!(CoreMaybeUninit<()>, CoreMaybeUninit<u8>);

// SAFETY: `ManuallyDrop<T>` has the same layout as `T` [1]. This strongly
// implies, but does not guarantee, that it contains `UnsafeCell`s covering the
// same byte ranges as in `T`. However, it also implements `Defer<Target = T>`
// [2], which provides the ability to convert `&ManuallyDrop<T> -> &T`. This,
// combined with having the same size as `T`, implies that `ManuallyDrop<T>`
// exactly contains a `T` with the same fields and `UnsafeCell`s covering the
// same byte ranges, or else the `Deref` impl would permit safe code to obtain
// different shared references to the same region of memory with different
// `UnsafeCell` coverage, which would in turn permit interior mutation that
// would violate the invariants of a shared reference.
//
// [1] Per https://doc.rust-lang.org/1.85.0/std/mem/struct.ManuallyDrop.html:
//
//   `ManuallyDrop<T>` is guaranteed to have the same layout and bit validity as
//   `T`
//
// [2] https://doc.rust-lang.org/1.85.0/std/mem/struct.ManuallyDrop.html#impl-Deref-for-ManuallyDrop%3CT%3E
const _: () = unsafe { unsafe_impl!(T: ?Sized + Immutable => Immutable for ManuallyDrop<T>) };

impl_for_transmute_from!(T: ?Sized + TryFromBytes => TryFromBytes for ManuallyDrop<T>[T]);
impl_for_transmute_from!(T: ?Sized + FromZeros => FromZeros for ManuallyDrop<T>[T]);
impl_for_transmute_from!(T: ?Sized + FromBytes => FromBytes for ManuallyDrop<T>[T]);
impl_for_transmute_from!(T: ?Sized + IntoBytes => IntoBytes for ManuallyDrop<T>[T]);
// SAFETY: `ManuallyDrop<T>` has the same layout as `T` [1], and thus has the
// same alignment as `T`.
//
// [1] Per https://doc.rust-lang.org/1.81.0/std/mem/struct.ManuallyDrop.html:
//
//   `ManuallyDrop<T>` is guaranteed to have the same layout and bit validity as
//   `T`
const _: () = unsafe { unsafe_impl!(T: ?Sized + Unaligned => Unaligned for ManuallyDrop<T>) };
assert_unaligned!(ManuallyDrop<()>, ManuallyDrop<u8>);

const _: () = {
    #[allow(
        non_camel_case_types,
        missing_copy_implementations,
        missing_debug_implementations,
        missing_docs
    )]
    pub enum value {}

    // SAFETY: See safety comment on `ProjectToTag`.
    unsafe impl<T: ?Sized, Client> HasTag<Client> for ManuallyDrop<T> {
        #[inline]
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }

        type Tag = ();

        // SAFETY: It is trivially sound to project any pointer to a pointer to
        // a type of size zero and alignment 1 (which `()` is [1]). Such a
        // pointer will trivially satisfy its aliasing and validity requirements
        // (since it has a zero-sized referent), and its alignment requirement
        // (since it is aligned to 1).
        //
        // [1] Per https://doc.rust-lang.org/1.92.0/reference/type-layout.html#r-layout.tuple.unit:
        //
        //     [T]he unit tuple (`()`)... is guaranteed as a zero-sized type to
        //     have a size of 0 and an alignment of 1.
        type ProjectToTag = crate::pointer::cast::CastToUnit;
    }

    // SAFETY: `ManuallyDrop<T>` has a field of type `T` at offset `0` without
    // any safety invariants beyond those of `T`.  Its existence is not
    // explicitly documented, but it can be inferred; per [1] `ManuallyDrop<T>`
    // has the same size and bit validity as `T`. This field is not literally
    // public, but is effectively so; the field can be transparently:
    //
    //  - initialized via `ManuallyDrop::new`
    //  - moved via `ManuallyDrop::into_inner`
    //  - referenced via `ManuallyDrop::deref`
    //  - exclusively referenced via `ManuallyDrop::deref_mut`
    //
    // We call this field `value`, both because that is both the name of this
    // private field, and because it is the name it is referred to in the public
    // documentation of `ManuallyDrop::new`, `ManuallyDrop::into_inner`,
    // `ManuallyDrop::take` and `ManuallyDrop::drop`.
    unsafe impl<T: ?Sized, Client>
        HasField<Client, value, { crate::STRUCT_VARIANT_ID }, { crate::ident_id!(value) }>
        for ManuallyDrop<T>
    {
        #[inline]
        fn only_derive_is_allowed_to_implement_this_trait()
        where
            Self: Sized,
        {
        }

        type Type = T;

        #[inline(always)]
        fn project(slf: PtrInner<'_, Self>) -> *mut T {
            // SAFETY: `ManuallyDrop<T>` has the same layout and bit validity as
            // `T` [1].
            //
            // [1] Per https://doc.rust-lang.org/1.85.0/std/mem/struct.ManuallyDrop.html:
            //
            //   `ManuallyDrop<T>` is guaranteed to have the same layout and bit
            //   validity as `T`
            #[allow(clippy::as_conversions)]
            return slf.as_ptr() as *mut T;
        }
    }
};

impl_for_transmute_from!(T: ?Sized + TryFromBytes => TryFromBytes for Cell<T>[T]);
impl_for_transmute_from!(T: ?Sized + FromZeros => FromZeros for Cell<T>[T]);
impl_for_transmute_from!(T: ?Sized + FromBytes => FromBytes for Cell<T>[T]);
impl_for_transmute_from!(T: ?Sized + IntoBytes => IntoBytes for Cell<T>[T]);
// SAFETY: `Cell<T>` has the same in-memory representation as `T` [1], and thus
// has the same alignment as `T`.
//
// [1] Per https://doc.rust-lang.org/1.81.0/core/cell/struct.Cell.html#memory-layout:
//
//   `Cell<T>` has the same in-memory representation as its inner type `T`.
const _: () = unsafe { unsafe_impl!(T: ?Sized + Unaligned => Unaligned for Cell<T>) };

impl_for_transmute_from!(T: ?Sized + FromZeros => FromZeros for UnsafeCell<T>[T]);
impl_for_transmute_from!(T: ?Sized + FromBytes => FromBytes for UnsafeCell<T>[T]);
impl_for_transmute_from!(T: ?Sized + IntoBytes => IntoBytes for UnsafeCell<T>[T]);
// SAFETY: `UnsafeCell<T>` has the same in-memory representation as `T` [1], and
// thus has the same alignment as `T`.
//
// [1] Per https://doc.rust-lang.org/1.81.0/core/cell/struct.UnsafeCell.html#memory-layout:
//
//   `UnsafeCell<T>` has the same in-memory representation as its inner type
//   `T`.
const _: () = unsafe { unsafe_impl!(T: ?Sized + Unaligned => Unaligned for UnsafeCell<T>) };
assert_unaligned!(UnsafeCell<()>, UnsafeCell<u8>);

// SAFETY: See safety comment in `is_safe` impl.
unsafe impl<T: TryFromBytes + ?Sized> TryFromBytes for UnsafeCell<T> {
    #[allow(clippy::missing_inline_in_public_items)]
    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized,
    {
    }

    #[inline(always)]
    fn is_safe<A>(candidate: Maybe<'_, Self, A>) -> bool
    where
        A: invariant::Alignment,
    {
        T::is_safe(candidate.transmute::<_, _, BecauseImmutable>())
    }
}

// SAFETY: Per the reference [1]:
//
//   An array of `[T; N]` has a size of `size_of::<T>() * N` and the same
//   alignment of `T`. Arrays are laid out so that the zero-based `nth` element
//   of the array is offset from the start of the array by `n * size_of::<T>()`
//   bytes.
//
//   ...
//
//   Slices have the same layout as the section of the array they slice.
//
// In other words, the layout of a `[T]` or `[T; N]` is a sequence of `T`s laid
// out back-to-back with no bytes in between. Therefore, `[T]` or `[T; N]` are
// `Immutable`, `TryFromBytes`, `FromZeros`, `FromBytes`, and `IntoBytes` if `T`
// is (respectively). Furthermore, since an array/slice has "the same alignment
// of `T`", `[T]` and `[T; N]` are `Unaligned` if `T` is.
//
// Note that we don't `assert_unaligned!` for slice types because
// `assert_unaligned!` uses `align_of`, which only works for `Sized` types.
//
// [1] https://doc.rust-lang.org/1.81.0/reference/type-layout.html#array-layout
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(const N: usize, T: Immutable => Immutable for [T; N]);
    unsafe_impl!(const N: usize, T: TryFromBytes => TryFromBytes for [T; N]; |c| {
        let c: Ptr<'_, [ReadOnly<T>; N], _> = c.cast::<_, crate::pointer::cast::CastSized, _>();
        let c: Ptr<'_, [ReadOnly<T>], _> = c.as_slice();
        let c: Ptr<'_, ReadOnly<[T]>, _> = c.cast::<_, crate::pointer::cast::CastUnsized, _>();

        // Note that this call may panic, but it would still be sound even if it
        // did. `is_safe` does not promise that it will not panic (in fact, it
        // explicitly warns that it's a possibility), and we have not violated
        // any safety invariants that we must fix before returning.
        <[T] as TryFromBytes>::is_safe(c)
    });
    unsafe_impl!(const N: usize, T: FromZeros => FromZeros for [T; N]);
    unsafe_impl!(const N: usize, T: FromBytes => FromBytes for [T; N]);
    unsafe_impl!(const N: usize, T: IntoBytes => IntoBytes for [T; N]);
    unsafe_impl!(const N: usize, T: Unaligned => Unaligned for [T; N]);
    assert_unaligned!([(); 0], [(); 1], [u8; 0], [u8; 1]);
    unsafe_impl!(T: Immutable => Immutable for [T]);
    unsafe_impl!(T: TryFromBytes => TryFromBytes for [T]; |c| {
        let c: Ptr<'_, [ReadOnly<T>], _> = c.cast::<_, crate::pointer::cast::CastUnsized, _>();

        // SAFETY: Per the reference [1]:
        //
        //   An array of `[T; N]` has a size of `size_of::<T>() * N` and the
        //   same alignment of `T`. Arrays are laid out so that the zero-based
        //   `nth` element of the array is offset from the start of the array by
        //   `n * size_of::<T>()` bytes.
        //
        //   ...
        //
        //   Slices have the same layout as the section of the array they slice.
        //
        // In other words, the layout of a `[T] is a sequence of `T`s laid out
        // back-to-back with no bytes in between. If all elements in `candidate`
        // are `is_safe`, so too is `candidate`.
        //
        // Note that any of the below calls may panic, but it would still be
        // sound even if it did. `is_safe` does not promise that it will not
        // panic (in fact, it explicitly warns that it's a possibility), and we
        // have not violated any safety invariants that we must fix before
        // returning.
        c.iter().all(<T as TryFromBytes>::is_safe)
    });
    unsafe_impl!(T: FromZeros => FromZeros for [T]);
    unsafe_impl!(T: FromBytes => FromBytes for [T]);
    unsafe_impl!(T: IntoBytes => IntoBytes for [T]);
    unsafe_impl!(T: Unaligned => Unaligned for [T]);
};

// SAFETY:
// - `Immutable`: Raw pointers do not contain any `UnsafeCell`s.
// - `FromZeros`: For thin pointers (note that `T: Sized`), the zero pointer is
//   considered "null". [1] No operations which require provenance are legal on
//   null pointers, so this is not a footgun.
// - `TryFromBytes`: By the same reasoning as for `FromZeroes`, we can implement
//   `TryFromBytes` for thin pointers provided that
//   [`TryFromBytes::is_safe`] only produces `true` for zeroed bytes.
//
// NOTE(#170): Implementing `FromBytes` and `IntoBytes` for raw pointers would
// be sound, but carries provenance footguns. We want to support `FromBytes` and
// `IntoBytes` for raw pointers eventually, but we are holding off until we can
// figure out how to address those footguns.
//
// [1] Per https://doc.rust-lang.org/1.81.0/std/ptr/fn.null.html:
//
//   Creates a null raw pointer.
//
//   This function is equivalent to zero-initializing the pointer:
//   `MaybeUninit::<*const T>::zeroed().assume_init()`.
//
//   The resulting pointer has the address 0.
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(T: ?Sized => Immutable for *const T);
    unsafe_impl!(T: ?Sized => Immutable for *mut T);
    unsafe_impl!(T => TryFromBytes for *const T; |c| pointer::is_zeroed(c));
    unsafe_impl!(T => FromZeros for *const T);
    unsafe_impl!(T => TryFromBytes for *mut T; |c| pointer::is_zeroed(c));
    unsafe_impl!(T => FromZeros for *mut T);
};

// SAFETY: `NonNull<T>` self-evidently does not contain `UnsafeCell`s. This is
// not a proof, but we are accepting this as a known risk per #1358.
const _: () = unsafe { unsafe_impl!(T: ?Sized => Immutable for NonNull<T>) };

// SAFETY: Reference types do not contain any `UnsafeCell`s.
#[allow(clippy::multiple_unsafe_ops_per_block)]
const _: () = unsafe {
    unsafe_impl!(T: ?Sized => Immutable for &'_ T);
    unsafe_impl!(T: ?Sized => Immutable for &'_ mut T);
};

// SAFETY: `Option` is not `#[non_exhaustive]` [1], which means that the types
// in its variants cannot change, and no new variants can be added. `Option<T>`
// does not contain any `UnsafeCell`s outside of `T`. [1]
//
// [1] https://doc.rust-lang.org/core/option/enum.Option.html
const _: () = unsafe { unsafe_impl!(T: Immutable => Immutable for Option<T>) };

mod tuples {
    use super::*;

    /// Generates various trait implementations for tuples.
    ///
    /// # Safety
    ///
    /// `impl_tuple!` should be provided name-number pairs, where each number is
    /// the ordinal of the preceding type name.
    macro_rules! impl_tuple {
        // Entry point.
        ($($T:ident $I:tt),+ $(,)?) => {
            crate::util::macros::__unsafe();
            impl_tuple!(@all [] [$($T $I)+]);
        };

        // Build up the set of tuple types (i.e., `(A,)`, `(A, B)`, `(A, B, C)`,
        // etc.) Trait implementations that do not depend on field index may be
        // added to this branch.
        (@all [$($head_T:ident $head_I:tt)*] [$next_T:ident $next_I:tt $($tail:tt)*]) => {
            // SAFETY: If all fields of the tuple `Self` are `Immutable`, so too is `Self`.
            unsafe_impl!($($head_T: Immutable,)* $next_T: Immutable => Immutable for ($($head_T,)* $next_T,));

            // SAFETY: If all fields in `c` are `is_safe`, so too is `c`.
            unsafe_impl!($($head_T: TryFromBytes,)* $next_T: TryFromBytes => TryFromBytes for ($($head_T,)* $next_T,); |c| {
                let mut c = c;
                $(TryFromBytes::is_safe(into_inner!(c.reborrow().project::<crate::project_clients::TryFromBytesDerive, _, { crate::STRUCT_VARIANT_ID }, { crate::ident_id!($head_I) }>())) &&)*
                    TryFromBytes::is_safe(into_inner!(c.reborrow().project::<crate::project_clients::TryFromBytesDerive, _, { crate::STRUCT_VARIANT_ID }, { crate::ident_id!($next_I) }>()))
            });

            // SAFETY: If all fields in `Self` are `FromZeros`, so too is `Self`.
            unsafe_impl!($($head_T: FromZeros,)* $next_T: FromZeros => FromZeros for ($($head_T,)* $next_T,));

            // SAFETY: If all fields in `Self` are `FromBytes`, so too is `Self`.
            unsafe_impl!($($head_T: FromBytes,)* $next_T: FromBytes => FromBytes for ($($head_T,)* $next_T,));

            // SAFETY: See safety comment on `ProjectToTag`.
            unsafe impl<Client, $($head_T,)* $next_T> crate::HasTag<Client>
                for ($($head_T,)* $next_T,)
            {
                #[inline]
                fn only_derive_is_allowed_to_implement_this_trait()
                where
                    Self: Sized
                {}

                type Tag = ();

                // SAFETY: It is trivially sound to project any pointer to a
                // pointer to a type of size zero and alignment 1 (which `()` is
                // [1]). Such a pointer will trivially satisfy its aliasing and
                // validity requirements (since it has a zero-sized referent),
                // and its alignment requirement (since it is aligned to 1).
                //
                // [1] Per https://doc.rust-lang.org/1.92.0/reference/type-layout.html#r-layout.tuple.unit:
                //
                //     [T]he unit tuple (`()`)... is guaranteed as a zero-sized
                //     type to have a size of 0 and an alignment of 1.
                type ProjectToTag = crate::pointer::cast::CastToUnit;
            }

            // Generate impls that depend on tuple index.
            impl_tuple!(@variants
                [$($head_T $head_I)* $next_T $next_I]
                []
                [$($head_T $head_I)* $next_T $next_I]
            );

            // Recurse to next tuple size
            impl_tuple!(@all [$($head_T $head_I)* $next_T $next_I] [$($tail)*]);
        };
        (@all [$($head_T:ident $head_I:tt)*] []) => {};

        // Emit trait implementations that depend on field index.
        (@variants
            // The full tuple definition in type–index pairs.
            [$($AllT:ident $AllI:tt)+]
            // Types before the current index.
            [$($BeforeT:ident)*]
            // The types and indices at and after the current index.
            [$CurrT:ident $CurrI:tt $($AfterT:ident $AfterI:tt)*]
        ) => {
            // SAFETY:
            // - `Self` is a struct (albeit anonymous), so `VARIANT_ID` is
            //   `STRUCT_VARIANT_ID`.
            // - `$CurrI` is the field at index `$CurrI`, so `FIELD_ID` is
            //   `zerocopy::ident_id!($CurrI)`
            // - `()` has the same visibility as the `.$CurrI` field (ie, `.0`,
            //   `.1`, etc)
            // - `Type` has the same type as `$CurrI`; i.e., `$CurrT`.
            unsafe impl<Client, $($AllT),+> crate::HasField<
                Client,
                (),
                { crate::STRUCT_VARIANT_ID },
                { crate::ident_id!($CurrI)}
            > for ($($AllT,)+) {
                #[inline]
                fn only_derive_is_allowed_to_implement_this_trait()
                where
                    Self: Sized
                {}

                type Type = $CurrT;

                #[inline(always)]
                fn project(slf: crate::PtrInner<'_, Self>) -> *mut Self::Type {
                    let slf = slf.as_non_null().as_ptr();
                    // SAFETY: `PtrInner` promises it references either a zero-sized
                    // byte range, or else will reference a byte range that is
                    // entirely contained within an allocated object. In either
                    // case, this guarantees that `(*slf).$CurrI` is in-bounds of
                    // `slf`.
                    unsafe { core::ptr::addr_of_mut!((*slf).$CurrI) }
                }
            }

            // SAFETY: See comments on items.
            unsafe impl<Client, Aliasing, Alignment, $($AllT),+> crate::ProjectField<
                Client,
                (),
                (Aliasing, Alignment, crate::invariant::Uninit),
                { crate::STRUCT_VARIANT_ID },
                { crate::ident_id!($CurrI)}
            > for ($($AllT,)+)
            where
                Aliasing: crate::invariant::Aliasing,
                Alignment: crate::invariant::Alignment,
            {
                #[inline]
                fn only_derive_is_allowed_to_implement_this_trait()
                where
                    Self: Sized
                {}

                // SAFETY: Tuples are product types whose fields are
                // well-aligned, so projection preserves both the alignment and
                // validity invariants of the outer pointer.
                type Invariants = (Aliasing, Alignment, crate::invariant::Uninit);

                // SAFETY: Tuples are product types and so projection is infallible;
                type Error = core::convert::Infallible;
            }

            // SAFETY: See comments on items.
            unsafe impl<Client, Aliasing, Alignment, $($AllT),+> crate::ProjectField<
                Client,
                (),
                (Aliasing, Alignment, crate::invariant::Initialized),
                { crate::STRUCT_VARIANT_ID },
                { crate::ident_id!($CurrI)}
            > for ($($AllT,)+)
            where
                Aliasing: crate::invariant::Aliasing,
                Alignment: crate::invariant::Alignment,
            {
                #[inline]
                fn only_derive_is_allowed_to_implement_this_trait()
                where
                    Self: Sized
                {}

                // SAFETY: Tuples are product types whose fields are
                // well-aligned, so projection preserves both the alignment and
                // validity invariants of the outer pointer.
                type Invariants = (Aliasing, Alignment, crate::invariant::Initialized);

                // SAFETY: Tuples are product types and so projection is infallible;
                type Error = core::convert::Infallible;
            }

            // SAFETY: See comments on items.
            unsafe impl<Client, Aliasing, Alignment, $($AllT),+> crate::ProjectField<
                Client,
                (),
                (Aliasing, Alignment, crate::invariant::Safe),
                { crate::STRUCT_VARIANT_ID },
                { crate::ident_id!($CurrI)}
            > for ($($AllT,)+)
            where
                Aliasing: crate::invariant::Aliasing,
                Alignment: crate::invariant::Alignment,
            {
                #[inline]
                fn only_derive_is_allowed_to_implement_this_trait()
                where
                    Self: Sized
                {}

                // SAFETY: Tuples are product types whose fields are
                // well-aligned, so projection preserves both the alignment and
                // validity invariants of the outer pointer.
                type Invariants = (Aliasing, Alignment, crate::invariant::Safe);

                // SAFETY: Tuples are product types and so projection is infallible;
                type Error = core::convert::Infallible;
            }

            // Recurse to the next index.
            impl_tuple!(@variants [$($AllT $AllI)+] [$($BeforeT)* $CurrT] [$($AfterT $AfterI)*]);
        };
        (@variants [$($AllT:ident $AllI:tt)+] [$($BeforeT:ident)*] []) => {};
    }

    // SAFETY: `impl_tuple` is provided name-number pairs, where number is the
    // ordinal of the name.
    #[allow(clippy::multiple_unsafe_ops_per_block)]
    const _: () = unsafe {
        impl_tuple! {
            A 0,
            B 1,
            C 2,
            D 3,
            E 4,
            F 5,
            G 6,
            H 7,
            I 8,
            J 9,
            K 10,
            L 11,
            M 12,
            N 13,
            O 14,
            P 15,
            Q 16,
            R 17,
            S 18,
            T 19,
            U 20,
            V 21,
            W 22,
            X 23,
            Y 24,
            Z 25,
        };
    };
}

// SIMD support
//
// Per the Unsafe Code Guidelines Reference [1]:
//
//   Packed SIMD vector types are `repr(simd)` homogeneous tuple-structs
//   containing `N` elements of type `T` where `N` is a power-of-two and the
//   size and alignment requirements of `T` are equal:
//
//   ```rust
//   #[repr(simd)]
//   struct Vector<T, N>(T_0, ..., T_(N - 1));
//   ```
//
//   ...
//
//   The size of `Vector` is `N * size_of::<T>()` and its alignment is an
//   implementation-defined function of `T` and `N` greater than or equal to
//   `align_of::<T>()`.
//
//   ...
//
//   Vector elements are laid out in source field order, enabling random access
//   to vector elements by reinterpreting the vector as an array:
//
//   ```rust
//   union U {
//      vec: Vector<T, N>,
//      arr: [T; N]
//   }
//
//   assert_eq!(size_of::<Vector<T, N>>(), size_of::<[T; N]>());
//   assert!(align_of::<Vector<T, N>>() >= align_of::<[T; N]>());
//
//   unsafe {
//     let u = U { vec: Vector<T, N>(t_0, ..., t_(N - 1)) };
//
//     assert_eq!(u.vec.0, u.arr[0]);
//     // ...
//     assert_eq!(u.vec.(N - 1), u.arr[N - 1]);
//   }
//   ```
//
// Given this background, we can observe that:
// - The size and bit pattern requirements of a SIMD type are equivalent to the
//   equivalent array type. Thus, for any SIMD type whose primitive `T` is
//   `Immutable`, `TryFromBytes`, `FromZeros`, `FromBytes`, or `IntoBytes`, that
//   SIMD type is also `Immutable`, `TryFromBytes`, `FromZeros`, `FromBytes`, or
//   `IntoBytes` respectively.
// - Since no upper bound is placed on the alignment, no SIMD type can be
//   guaranteed to be `Unaligned`.
//
// Also per [1]:
//
//   This chapter represents the consensus from issue #38. The statements in
//   here are not (yet) "guaranteed" not to change until an RFC ratifies them.
//
// See issue #38 [2]. While this behavior is not technically guaranteed, the
// likelihood that the behavior will change such that SIMD types are no longer
// `TryFromBytes`, `FromZeros`, `FromBytes`, or `IntoBytes` is next to zero, as
// that would defeat the entire purpose of SIMD types. Nonetheless, we put this
// behavior behind the `simd` Cargo feature, which requires consumers to opt
// into this stability hazard.
//
// [1] https://rust-lang.github.io/unsafe-code-guidelines/layout/packed-simd-vectors.html
// [2] https://github.com/rust-lang/unsafe-code-guidelines/issues/38
#[cfg(feature = "simd")]

mod simd {
    /// Defines a module which implements `TryFromBytes`, `FromZeros`,
    /// `FromBytes`, and `IntoBytes` for a set of types from a module in
    /// `core::arch`.
    ///
    /// `$arch` is both the name of the defined module and the name of the
    /// module in `core::arch`, and `$typ` is the list of items from that module
    /// to implement `FromZeros`, `FromBytes`, and `IntoBytes` for.
    #[allow(unused_macros)] // `allow(unused_macros)` is needed because some
                            // target/feature combinations don't emit any impls
                            // and thus don't use this macro.
    macro_rules! simd_arch_mod {
        ($(#[cfg $cfg:tt])* $(#[cfg_attr $cfg_attr:tt])? $arch:ident, $mod:ident, $($typ:ident),*) => {
            $(#[cfg $cfg])*
            #[cfg_attr(doc_cfg, doc(cfg $($cfg)*))]
            $(#[cfg_attr $cfg_attr])?
            mod $mod {
                use core::arch::$arch::{$($typ),*};

                use crate::*;
                impl_known_layout!($($typ),*);
                // SAFETY: See comment on module definition for justification.
                #[allow(clippy::multiple_unsafe_ops_per_block)]
                const _: () = unsafe {
                    $( unsafe_impl!($typ: Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes); )*
                };
            }
        };
    }

    #[rustfmt::skip]
    const _: () = {
        simd_arch_mod!(
            #[cfg(target_arch = "x86")]
            x86, x86, __m128, __m128d, __m128i, __m256, __m256d, __m256i
        );
        #[cfg(not(no_zerocopy_simd_x86_avx12_1_89_0))]
        simd_arch_mod!(
            #[cfg(target_arch = "x86")]
            #[cfg_attr(doc_cfg, doc(cfg(rust = "1.89.0")))]
            x86, x86_nightly, __m512bh, __m512, __m512d, __m512i
        );
        simd_arch_mod!(
            #[cfg(target_arch = "x86_64")]
            x86_64, x86_64, __m128, __m128d, __m128i, __m256, __m256d, __m256i
        );
        #[cfg(not(no_zerocopy_simd_x86_avx12_1_89_0))]
        simd_arch_mod!(
            #[cfg(target_arch = "x86_64")]
            #[cfg_attr(doc_cfg, doc(cfg(rust = "1.89.0")))]
            x86_64, x86_64_nightly, __m512bh, __m512, __m512d, __m512i
        );
        simd_arch_mod!(
            #[cfg(target_arch = "wasm32")]
            wasm32, wasm32, v128
        );
        simd_arch_mod!(
            #[cfg(all(feature = "simd-nightly", target_arch = "powerpc"))]
            powerpc, powerpc, vector_bool_long, vector_double, vector_signed_long, vector_unsigned_long
        );
        simd_arch_mod!(
            #[cfg(all(feature = "simd-nightly", target_arch = "powerpc64"))]
            powerpc64, powerpc64, vector_bool_long, vector_double, vector_signed_long, vector_unsigned_long
        );
        // NOTE: NEON intrinsics were broken on big-endian platforms from their stabilization up to
        // Rust 1.87. (Context in https://github.com/rust-lang/stdarch/issues/1484). Support is
        // split in two different version ranges on top of the base configuration, requiring either
        // little endian or the more recent version to be detected as well.
        #[cfg(not(no_zerocopy_aarch64_simd_1_59_0))]
        simd_arch_mod!(
            #[cfg(all(
                target_arch = "aarch64",
                any(
                    target_endian = "little",
                    not(no_zerocopy_aarch64_simd_be_1_87_0)
                )
            ))]
            #[cfg_attr(
                doc_cfg,
                doc(cfg(all(target_arch = "aarch64", any(
                    all(rust = "1.59.0", target_endian = "little"),
                    rust = "1.87.0",
                ))))
            )]
            aarch64, aarch64, float32x2_t, float32x4_t, float64x1_t, float64x2_t, int8x8_t, int8x8x2_t,
            int8x8x3_t, int8x8x4_t, int8x16_t, int8x16x2_t, int8x16x3_t, int8x16x4_t, int16x4_t,
            int16x8_t, int32x2_t, int32x4_t, int64x1_t, int64x2_t, poly8x8_t, poly8x8x2_t, poly8x8x3_t,
            poly8x8x4_t, poly8x16_t, poly8x16x2_t, poly8x16x3_t, poly8x16x4_t, poly16x4_t, poly16x8_t,
            poly64x1_t, poly64x2_t, uint8x8_t, uint8x8x2_t, uint8x8x3_t, uint8x8x4_t, uint8x16_t,
            uint8x16x2_t, uint8x16x3_t, uint8x16x4_t, uint16x4_t, uint16x4x2_t, uint16x4x3_t,
            uint16x4x4_t, uint16x8_t, uint32x2_t, uint32x4_t, uint64x1_t, uint64x2_t
        );
    };
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/unit/src/impls/tests_1392.rs"));
