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

/// Safely transmutes a value of one type to a value of another type of the same
/// size.
///
/// This macro behaves like an invocation of this function:
///
/// ```ignore
/// const fn transmute<Src, Dst>(src: Src) -> Dst
/// where
///     Src: IntoBytes,
///     Dst: FromBytes,
///     size_of::<Src>() == size_of::<Dst>(),
/// {
/// # /*
///     ...
/// # */
/// }
/// ```
///
/// However, unlike a function, this macro can only be invoked when the types of
/// `Src` and `Dst` are completely concrete. The types `Src` and `Dst` are
/// inferred from the calling context; they cannot be explicitly specified in
/// the macro invocation.
///
/// Note that the `Src` produced by the expression `$e` will *not* be dropped.
/// Semantically, its bits will be copied into a new value of type `Dst`, the
/// original `Src` will be forgotten, and the value of type `Dst` will be
/// returned.
///
/// # `#![allow(shrink)]`
///
/// If `#![allow(shrink)]` is provided, `transmute!` additionally supports
/// transmutations that shrink the size of the value; e.g.:
///
/// ```
/// # use zerocopy::transmute;
/// let u: u32 = transmute!(#![allow(shrink)] 0u64);
/// assert_eq!(u, 0u32);
/// ```
///
/// # Examples
///
/// ```
/// # use zerocopy::transmute;
/// let one_dimensional: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
///
/// let two_dimensional: [[u8; 4]; 2] = transmute!(one_dimensional);
///
/// assert_eq!(two_dimensional, [[0, 1, 2, 3], [4, 5, 6, 7]]);
/// ```
///
/// # Use in `const` contexts
///
/// This macro can be invoked in `const` contexts.
///
#[doc = codegen_section!(
    header = "h2",
    bench = "transmute",
    format = "coco_static_size",
)]
#[macro_export]
macro_rules! transmute {
    // NOTE: This must be a macro (rather than a function with trait bounds)
    // because there's no way, in a generic context, to enforce that two types
    // have the same size. `core::mem::transmute` uses compiler magic to enforce
    // this so long as the types are concrete.
    (#![allow(shrink)] $e:expr) => {{
        let mut e = $e;
        if false {
            // This branch, though never taken, ensures that the type of `e` is
            // `IntoBytes` and that the type of the  outer macro invocation
            // expression is `FromBytes`.

            fn transmute<Src, Dst>(src: Src) -> Dst
            where
                Src: $crate::IntoBytes,
                Dst: $crate::FromBytes,
            {
                let _ = src;
                loop {}
            }
            loop {}
            #[allow(unreachable_code)]
            transmute(e)
        } else {
            use $crate::util::macro_util::core_reexport::mem::ManuallyDrop;

            // NOTE: `repr(packed)` is important! It ensures that the size of
            // `Transmute` won't be rounded up to accommodate `Src`'s or `Dst`'s
            // alignment, which would break the size comparison logic below.
            //
            // As an example of why this is problematic, consider `Src = [u8;
            // 5]`, `Dst = u32`. The total size of `Transmute<Src, Dst>` would
            // be 8, and so we would reject a `[u8; 5]` to `u32` transmute as
            // being size-increasing, which it isn't.
            #[repr(C, packed)]
            union Transmute<Src, Dst> {
                src: ManuallyDrop<Src>,
                dst: ManuallyDrop<Dst>,
            }

            // SAFETY: `Transmute` is a `repr(C)` union whose `src` field has
            // type `ManuallyDrop<Src>`. Thus, the `src` field starts at byte
            // offset 0 within `Transmute` [1]. `ManuallyDrop<T>` has the same
            // layout and bit validity as `T`, so it is sound to transmute `Src`
            // to `Transmute`.
            //
            // [1] https://doc.rust-lang.org/1.85.0/reference/type-layout.html#reprc-unions
            //
            // [2] Per https://doc.rust-lang.org/1.85.0/std/mem/struct.ManuallyDrop.html:
            //
            //   `ManuallyDrop<T>` is guaranteed to have the same layout and bit
            //   validity as `T`
            let u: Transmute<_, _> = unsafe {
                // Clippy: We can't annotate the types; this macro is designed
                // to infer the types from the calling context.
                #[allow(clippy::missing_transmute_annotations)]
                $crate::util::macro_util::core_reexport::mem::transmute(e)
            };

            if false {
                // SAFETY: This code is never executed.
                e = ManuallyDrop::into_inner(unsafe { u.src });
                // Suppress the `unused_assignments` lint on the previous line.
                let _ = e;
                loop {}
            } else {
                // SAFETY: Per the safety comment on `let u` above, the `dst`
                // field in `Transmute` starts at byte offset 0, and has the
                // same layout and bit validity as `Dst`.
                //
                // Transmuting `Src` to `Transmute<Src, Dst>` above using
                // `core::mem::transmute` ensures that `size_of::<Src>() ==
                // size_of::<Transmute<Src, Dst>>()`. A `#[repr(C, packed)]`
                // union has the maximum size of all of its fields [1], so this
                // is equivalent to `size_of::<Src>() >= size_of::<Dst>()`.
                //
                // The outer `if`'s `false` branch ensures that `Src: IntoBytes`
                // and `Dst: FromBytes`. This, combined with the size bound,
                // ensures that this transmute is sound.
                //
                // [1] Per https://doc.rust-lang.org/1.85.0/reference/type-layout.html#reprc-unions:
                //
                //   The union will have a size of the maximum size of all of
                //   its fields rounded to its alignment
                let dst = unsafe { u.dst };
                $crate::util::macro_util::must_use(ManuallyDrop::into_inner(dst))
            }
        }
    }};
    ($e:expr) => {{
        let e = $e;
        if false {
            // This branch, though never taken, ensures that the type of `e` is
            // `IntoBytes` and that the type of the  outer macro invocation
            // expression is `FromBytes`.

            fn transmute<Src, Dst>(src: Src) -> Dst
            where
                Src: $crate::IntoBytes,
                Dst: $crate::FromBytes,
            {
                let _ = src;
                loop {}
            }
            loop {}
            #[allow(unreachable_code)]
            transmute(e)
        } else {
            // SAFETY: `core::mem::transmute` ensures that the type of `e` and
            // the type of this macro invocation expression have the same size.
            // We know this transmute is safe thanks to the `IntoBytes` and
            // `FromBytes` bounds enforced by the `false` branch.
            let u = unsafe {
                // Clippy: We can't annotate the types; this macro is designed
                // to infer the types from the calling context.
                #[allow(clippy::missing_transmute_annotations, unnecessary_transmutes)]
                $crate::util::macro_util::core_reexport::mem::transmute(e)
            };
            $crate::util::macro_util::must_use(u)
        }
    }};
}

/// Safely transmutes a mutable or immutable reference of one type to an
/// immutable reference of another type of the same size and compatible
/// alignment.
///
/// This macro behaves like an invocation of this function:
///
/// ```ignore
/// fn transmute_ref<'src, 'dst, Src, Dst>(src: &'src Src) -> &'dst Dst
/// where
///     'src: 'dst,
///     Src: IntoBytes + Immutable + ?Sized,
///     Dst: FromBytes + Immutable + ?Sized,
///     align_of::<Src>() >= align_of::<Dst>(),
///     size_compatible::<Src, Dst>(),
/// {
/// # /*
///     ...
/// # */
/// }
/// ```
///
/// The types `Src` and `Dst` are inferred from the calling context; they cannot
/// be explicitly specified in the macro invocation.
///
/// # Size compatibility
///
/// `transmute_ref!` supports transmuting between `Sized` types, between unsized
/// (i.e., `?Sized`) types, and from a `Sized` type to an unsized type. It
/// supports any transmutation that preserves the number of bytes of the
/// referent, even if doing so requires updating the metadata stored in an
/// unsized "fat" reference:
///
/// ```
/// # use zerocopy::transmute_ref;
/// # use core::mem::size_of_val; // Not in the prelude on our MSRV
/// let src: &[[u8; 2]] = &[[0, 1], [2, 3]][..];
/// let dst: &[u8] = transmute_ref!(src);
///
/// assert_eq!(src.len(), 2);
/// assert_eq!(dst.len(), 4);
/// assert_eq!(dst, [0, 1, 2, 3]);
/// assert_eq!(size_of_val(src), size_of_val(dst));
/// ```
///
/// # Errors
///
/// Violations of the alignment and size compatibility checks are detected
/// *after* the compiler performs monomorphization. This has two important
/// consequences.
///
/// First, it means that generic code will *never* fail these conditions:
///
/// ```
/// # use zerocopy::{transmute_ref, FromBytes, IntoBytes, Immutable};
/// fn transmute_ref<Src, Dst>(src: &Src) -> &Dst
/// where
///     Src: IntoBytes + Immutable,
///     Dst: FromBytes + Immutable,
/// {
///     transmute_ref!(src)
/// }
/// ```
///
/// Instead, failures will only be detected once generic code is instantiated
/// with concrete types:
///
/// ```compile_fail,E0080
/// # use zerocopy::{transmute_ref, FromBytes, IntoBytes, Immutable};
/// #
/// # fn transmute_ref<Src, Dst>(src: &Src) -> &Dst
/// # where
/// #     Src: IntoBytes + Immutable,
/// #     Dst: FromBytes + Immutable,
/// # {
/// #     transmute_ref!(src)
/// # }
/// let src: &u16 = &0;
/// let dst: &u8 = transmute_ref(src);
/// ```
///
/// Second, the fact that violations are detected after monomorphization means
/// that `cargo check` will usually not detect errors, even when types are
/// concrete. Instead, `cargo build` must be used to detect such errors.
///
/// # Examples
///
/// Transmuting between `Sized` types:
///
/// ```
/// # use zerocopy::transmute_ref;
/// let one_dimensional: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
///
/// let two_dimensional: &[[u8; 4]; 2] = transmute_ref!(&one_dimensional);
///
/// assert_eq!(two_dimensional, &[[0, 1, 2, 3], [4, 5, 6, 7]]);
/// ```
///
/// Transmuting between unsized types:
///
/// ```
/// # use {zerocopy::*, zerocopy_derive::*};
/// # type u16 = zerocopy::byteorder::native_endian::U16;
/// # type u32 = zerocopy::byteorder::native_endian::U32;
/// #[derive(KnownLayout, FromBytes, IntoBytes, Immutable)]
/// #[repr(C)]
/// struct SliceDst<T, U> {
///     t: T,
///     u: [U],
/// }
///
/// type Src = SliceDst<u32, u16>;
/// type Dst = SliceDst<u16, u8>;
///
/// let src = Src::ref_from_bytes(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
/// let dst: &Dst = transmute_ref!(src);
///
/// assert_eq!(src.t.as_bytes(), [0, 1, 2, 3]);
/// assert_eq!(src.u.len(), 2);
/// assert_eq!(src.u.as_bytes(), [4, 5, 6, 7]);
///
/// assert_eq!(dst.t.as_bytes(), [0, 1]);
/// assert_eq!(dst.u, [2, 3, 4, 5, 6, 7]);
/// ```
///
/// # Use in `const` contexts
///
/// This macro can be invoked in `const` contexts only when `Src: Sized` and
/// `Dst: Sized`.
///
#[doc = codegen_section!(
    header = "h2",
    bench = "transmute_ref",
    format = "coco",
    arity = 2,
    [
        open
        @index 1
        @title "Sized"
        @variant "static_size"
    ],
    [
        @index 2
        @title "Unsized"
        @variant "dynamic_size"
    ]
)]
#[macro_export]
macro_rules! transmute_ref {
    ($e:expr) => {{
        // NOTE: This must be a macro (rather than a function with trait bounds)
        // because there's no way, in a generic context, to enforce that two
        // types have the same size or alignment.

        // Ensure that the source type is a reference or a mutable reference
        // (note that mutable references are implicitly reborrowed here).
        let e: &_ = $e;

        #[allow(unused, clippy::diverging_sub_expression)]
        if false {
            // This branch, though never taken, ensures that the type of `e` is
            // `&T` where `T: IntoBytes + Immutable`, and that the type of this
            // macro expression is `&U` where `U: FromBytes + Immutable`.

            struct AssertSrcIsIntoBytes<'a, T: ?::core::marker::Sized + $crate::IntoBytes>(&'a T);
            struct AssertSrcIsImmutable<'a, T: ?::core::marker::Sized + $crate::Immutable>(&'a T);
            struct AssertDstIsFromBytes<'a, U: ?::core::marker::Sized + $crate::FromBytes>(&'a U);
            struct AssertDstIsImmutable<'a, T: ?::core::marker::Sized + $crate::Immutable>(&'a T);

            let _ = AssertSrcIsIntoBytes(e);
            let _ = AssertSrcIsImmutable(e);

            if true {
                #[allow(unused, unreachable_code)]
                let u = AssertDstIsFromBytes(loop {});
                u.0
            } else {
                #[allow(unused, unreachable_code)]
                let u = AssertDstIsImmutable(loop {});
                u.0
            }
        } else {
            use $crate::util::macro_util::TransmuteRefDst;
            let t = $crate::util::macro_util::Wrap::new(e);

            if false {
                // This branch exists solely to force the compiler to infer the
                // type of `Dst` *before* it attempts to resolve the method call
                // to `transmute_ref` in the `else` branch.
                //
                // Without this, if `Src` is `Sized` but `Dst` is `!Sized`, the
                // compiler will eagerly select the inherent impl of
                // `transmute_ref` (which requires `Dst: Sized`) because inherent
                // methods take priority over trait methods. It does this before
                // it realizes `Dst` is `!Sized`, leading to a compile error when
                // it checks the bounds later.
                //
                // By calling this helper (which returns `&Dst`), we force `Dst`
                // to be fully resolved. By the time it gets to the `else`
                // branch, the compiler knows `Dst` is `!Sized`, properly
                // disqualifies the inherent method, and falls back to the trait
                // implementation.
                t.transmute_ref_inference_helper()
            } else {
                // SAFETY: The outer `if false` branch ensures that:
                // - `Src: IntoBytes + Immutable`
                // - `Dst: FromBytes + Immutable`
                unsafe {
                    t.transmute_ref()
                }
            }
        }
    }}
}

/// Safely transmutes a mutable reference of one type to a mutable reference of
/// another type of the same size and compatible alignment.
///
/// This macro behaves like an invocation of this function:
///
/// ```ignore
/// const fn transmute_mut<'src, 'dst, Src, Dst>(src: &'src mut Src) -> &'dst mut Dst
/// where
///     'src: 'dst,
///     Src: FromBytes + IntoBytes + ?Sized,
///     Dst: FromBytes + IntoBytes + ?Sized,
///     align_of::<Src>() >= align_of::<Dst>(),
///     size_compatible::<Src, Dst>(),
/// {
/// # /*
///     ...
/// # */
/// }
/// ```
///
/// The types `Src` and `Dst` are inferred from the calling context; they cannot
/// be explicitly specified in the macro invocation.
///
/// # Size compatibility
///
/// `transmute_mut!` supports transmuting between `Sized` types, between unsized
/// (i.e., `?Sized`) types, and from a `Sized` type to an unsized type. It
/// supports any transmutation that preserves the number of bytes of the
/// referent, even if doing so requires updating the metadata stored in an
/// unsized "fat" reference:
///
/// ```
/// # use zerocopy::transmute_mut;
/// # use core::mem::size_of_val; // Not in the prelude on our MSRV
/// let src: &mut [[u8; 2]] = &mut [[0, 1], [2, 3]][..];
/// let dst: &mut [u8] = transmute_mut!(src);
///
/// assert_eq!(dst.len(), 4);
/// assert_eq!(dst, [0, 1, 2, 3]);
/// let dst_size = size_of_val(dst);
/// assert_eq!(src.len(), 2);
/// assert_eq!(size_of_val(src), dst_size);
/// ```
///
/// # Errors
///
/// Violations of the alignment and size compatibility checks are detected
/// *after* the compiler performs monomorphization. This has two important
/// consequences.
///
/// First, it means that generic code will *never* fail these conditions:
///
/// ```
/// # use zerocopy::{transmute_mut, FromBytes, IntoBytes, Immutable};
/// fn transmute_mut<Src, Dst>(src: &mut Src) -> &mut Dst
/// where
///     Src: FromBytes + IntoBytes,
///     Dst: FromBytes + IntoBytes,
/// {
///     transmute_mut!(src)
/// }
/// ```
///
/// Instead, failures will only be detected once generic code is instantiated
/// with concrete types:
///
/// ```compile_fail,E0080
/// # use zerocopy::{transmute_mut, FromBytes, IntoBytes, Immutable};
/// #
/// # fn transmute_mut<Src, Dst>(src: &mut Src) -> &mut Dst
/// # where
/// #     Src: FromBytes + IntoBytes,
/// #     Dst: FromBytes + IntoBytes,
/// # {
/// #     transmute_mut!(src)
/// # }
/// let src: &mut u16 = &mut 0;
/// let dst: &mut u8 = transmute_mut(src);
/// ```
///
/// Second, the fact that violations are detected after monomorphization means
/// that `cargo check` will usually not detect errors, even when types are
/// concrete. Instead, `cargo build` must be used to detect such errors.
///
///
/// # Examples
///
/// Transmuting between `Sized` types:
///
/// ```
/// # use zerocopy::transmute_mut;
/// let mut one_dimensional: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
///
/// let two_dimensional: &mut [[u8; 4]; 2] = transmute_mut!(&mut one_dimensional);
///
/// assert_eq!(two_dimensional, &[[0, 1, 2, 3], [4, 5, 6, 7]]);
///
/// two_dimensional.reverse();
///
/// assert_eq!(one_dimensional, [4, 5, 6, 7, 0, 1, 2, 3]);
/// ```
///
/// Transmuting between unsized types:
///
/// ```
/// # use {zerocopy::*, zerocopy_derive::*};
/// # type u16 = zerocopy::byteorder::native_endian::U16;
/// # type u32 = zerocopy::byteorder::native_endian::U32;
/// #[derive(KnownLayout, FromBytes, IntoBytes, Immutable)]
/// #[repr(C)]
/// struct SliceDst<T, U> {
///     t: T,
///     u: [U],
/// }
///
/// type Src = SliceDst<u32, u16>;
/// type Dst = SliceDst<u16, u8>;
///
/// let mut bytes = [0, 1, 2, 3, 4, 5, 6, 7];
/// let src = Src::mut_from_bytes(&mut bytes[..]).unwrap();
/// let dst: &mut Dst = transmute_mut!(src);
///
/// assert_eq!(dst.t.as_bytes(), [0, 1]);
/// assert_eq!(dst.u, [2, 3, 4, 5, 6, 7]);
///
/// assert_eq!(src.t.as_bytes(), [0, 1, 2, 3]);
/// assert_eq!(src.u.len(), 2);
/// assert_eq!(src.u.as_bytes(), [4, 5, 6, 7]);
/// ```
#[macro_export]
macro_rules! transmute_mut {
    ($e:expr) => {{
        // NOTE: This must be a macro (rather than a function with trait bounds)
        // because, for backwards-compatibility on v0.8.x, we use the autoref
        // specialization trick to dispatch to different `transmute_mut`
        // implementations: one which doesn't require `Src: KnownLayout + Dst:
        // KnownLayout` when `Src: Sized + Dst: Sized`, and one which requires
        // `KnownLayout` bounds otherwise.

        // Ensure that the source type is a mutable reference.
        let e: &mut _ = $e;

        #[allow(unused)]
        use $crate::util::macro_util::TransmuteMutDst as _;
        let t = $crate::util::macro_util::Wrap::new(e);
        if false {
            // This branch exists solely to force the compiler to infer the type
            // of `Dst` *before* it attempts to resolve the method call to
            // `transmute_mut` in the `else` branch.
            //
            // Without this, if `Src` is `Sized` but `Dst` is `!Sized`, the
            // compiler will eagerly select the inherent impl of `transmute_mut`
            // (which requires `Dst: Sized`) because inherent methods take
            // priority over trait methods. It does this before it realizes
            // `Dst` is `!Sized`, leading to a compile error when it checks the
            // bounds later.
            //
            // By calling this helper (which returns `&mut Dst`), we force `Dst`
            // to be fully resolved. By the time it gets to the `else` branch,
            // the compiler knows `Dst` is `!Sized`, properly disqualifies the
            // inherent method, and falls back to the trait implementation.
            t.transmute_mut_inference_helper()
        } else {
            t.transmute_mut()
        }
    }}
}

/// Conditionally transmutes a value of one type to a value of another type of
/// the same size.
///
/// This macro behaves like an invocation of this function:
///
/// ```ignore
/// fn try_transmute<Src, Dst>(src: Src) -> Result<Dst, ValidityError<Src, Dst>>
/// where
///     Src: IntoBytes,
///     Dst: TryFromBytes,
///     size_of::<Src>() == size_of::<Dst>(),
/// {
/// # /*
///     ...
/// # */
/// }
/// ```
///
/// However, unlike a function, this macro can only be invoked when the types of
/// `Src` and `Dst` are completely concrete. The types `Src` and `Dst` are
/// inferred from the calling context; they cannot be explicitly specified in
/// the macro invocation.
///
/// Note that the `Src` produced by the expression `$e` will *not* be dropped.
/// Semantically, its bits will be copied into a new value of type `Dst`, the
/// original `Src` will be forgotten, and the value of type `Dst` will be
/// returned.
///
/// # Examples
///
/// ```
/// # use zerocopy::*;
/// // 0u8 → bool = false
/// assert_eq!(try_transmute!(0u8), Ok(false));
///
/// // 1u8 → bool = true
///  assert_eq!(try_transmute!(1u8), Ok(true));
///
/// // 2u8 → bool = error
/// assert!(matches!(
///     try_transmute!(2u8),
///     Result::<bool, _>::Err(ValidityError { .. })
/// ));
/// ```
///
#[doc = codegen_section!(
    header = "h2",
    bench = "try_transmute",
    format = "coco_static_size",
)]
#[macro_export]
macro_rules! try_transmute {
    ($e:expr) => {{
        // NOTE: This must be a macro (rather than a function with trait bounds)
        // because there's no way, in a generic context, to enforce that two
        // types have the same size. `core::mem::transmute` uses compiler magic
        // to enforce this so long as the types are concrete.

        let e = $e;
        if false {
            // Check that the sizes of the source and destination types are
            // equal.

            // SAFETY: This code is never executed.
            Ok(unsafe {
                // Clippy: We can't annotate the types; this macro is designed
                // to infer the types from the calling context.
                #[allow(clippy::missing_transmute_annotations)]
                $crate::util::macro_util::core_reexport::mem::transmute(e)
            })
        } else {
            $crate::util::macro_util::try_transmute::<_, _>(e)
        }
    }}
}

/// Conditionally transmutes a mutable or immutable reference of one type to an
/// immutable reference of another type of the same size and compatible
/// alignment.
///
/// *Note that while the **value** of the referent is checked for validity at
/// runtime, the **size** and **alignment** are checked at compile time. For
/// conversions which are fallible with respect to size and alignment, see the
/// methods on [`TryFromBytes`].*
///
/// This macro behaves like an invocation of this function:
///
/// ```ignore
/// fn try_transmute_ref<Src, Dst>(src: &Src) -> Result<&Dst, ValidityError<&Src, Dst>>
/// where
///     Src: IntoBytes + Immutable + ?Sized,
///     Dst: TryFromBytes + Immutable + ?Sized,
///     align_of::<Src>() >= align_of::<Dst>(),
///     size_compatible::<Src, Dst>(),
/// {
/// # /*
///     ...
/// # */
/// }
/// ```
///
/// The types `Src` and `Dst` are inferred from the calling context; they cannot
/// be explicitly specified in the macro invocation.
///
/// [`TryFromBytes`]: crate::TryFromBytes
///
/// # Size compatibility
///
/// `try_transmute_ref!` supports transmuting between `Sized` types, between
/// unsized (i.e., `?Sized`) types, and from a `Sized` type to an unsized type.
/// It supports any transmutation that preserves the number of bytes of the
/// referent, even if doing so requires updating the metadata stored in an
/// unsized "fat" reference:
///
/// ```
/// # use zerocopy::try_transmute_ref;
/// # use core::mem::size_of_val; // Not in the prelude on our MSRV
/// let src: &[[u8; 2]] = &[[0, 1], [2, 3]][..];
/// let dst: &[u8] = try_transmute_ref!(src).unwrap();
///
/// assert_eq!(src.len(), 2);
/// assert_eq!(dst.len(), 4);
/// assert_eq!(dst, [0, 1, 2, 3]);
/// assert_eq!(size_of_val(src), size_of_val(dst));
/// ```
///
/// # Examples
///
/// Transmuting between `Sized` types:
///
/// ```
/// # use zerocopy::*;
/// // 0u8 → bool = false
/// assert_eq!(try_transmute_ref!(&0u8), Ok(&false));
///
/// // 1u8 → bool = true
///  assert_eq!(try_transmute_ref!(&1u8), Ok(&true));
///
/// // 2u8 → bool = error
/// assert!(matches!(
///     try_transmute_ref!(&2u8),
///     Result::<&bool, _>::Err(ValidityError { .. })
/// ));
/// ```
///
/// Transmuting between unsized types:
///
/// ```
/// # use {zerocopy::*, zerocopy_derive::*};
/// # type u16 = zerocopy::byteorder::native_endian::U16;
/// # type u32 = zerocopy::byteorder::native_endian::U32;
/// #[derive(KnownLayout, FromBytes, IntoBytes, Immutable)]
/// #[repr(C)]
/// struct SliceDst<T, U> {
///     t: T,
///     u: [U],
/// }
///
/// type Src = SliceDst<u32, u16>;
/// type Dst = SliceDst<u16, bool>;
///
/// let src = Src::ref_from_bytes(&[0, 1, 0, 1, 0, 1, 0, 1]).unwrap();
/// let dst: &Dst = try_transmute_ref!(src).unwrap();
///
/// assert_eq!(src.t.as_bytes(), [0, 1, 0, 1]);
/// assert_eq!(src.u.len(), 2);
/// assert_eq!(src.u.as_bytes(), [0, 1, 0, 1]);
///
/// assert_eq!(dst.t.as_bytes(), [0, 1]);
/// assert_eq!(dst.u, [false, true, false, true, false, true]);
/// ```
///
#[doc = codegen_section!(
    header = "h2",
    bench = "try_transmute_ref",
    format = "coco",
    arity = 2,
    [
        open
        @index 1
        @title "Sized"
        @variant "static_size"
    ],
    [
        @index 2
        @title "Unsized"
        @variant "dynamic_size"
    ]
)]
#[macro_export]
macro_rules! try_transmute_ref {
    ($e:expr) => {{
        // Ensure that the source type is a reference or a mutable reference
        // (note that mutable references are implicitly reborrowed here).
        let e: &_ = $e;

        #[allow(unused_imports)]
        use $crate::util::macro_util::TryTransmuteRefDst as _;
        let t = $crate::util::macro_util::Wrap::new(e);
        if false {
            // This branch exists solely to force the compiler to infer the type
            // of `Dst` *before* it attempts to resolve the method call to
            // `try_transmute_ref` in the `else` branch.
            //
            // Without this, if `Src` is `Sized` but `Dst` is `!Sized`, the
            // compiler will eagerly select the inherent impl of
            // `try_transmute_ref` (which requires `Dst: Sized`) because
            // inherent methods take priority over trait methods. It does this
            // before it realizes `Dst` is `!Sized`, leading to a compile error
            // when it checks the bounds later.
            //
            // By calling this helper (which returns `&Dst`), we force `Dst`
            // to be fully resolved. By the time it gets to the `else`
            // branch, the compiler knows `Dst` is `!Sized`, properly
            // disqualifies the inherent method, and falls back to the trait
            // implementation.
            Ok(t.transmute_ref_inference_helper())
        } else {
            t.try_transmute_ref()
        }
    }}
}

/// Conditionally transmutes a mutable reference of one type to a mutable
/// reference of another type of the same size and compatible alignment.
///
/// *Note that while the **value** of the referent is checked for validity at
/// runtime, the **size** and **alignment** are checked at compile time. For
/// conversions which are fallible with respect to size and alignment, see the
/// methods on [`TryFromBytes`].*
///
/// This macro behaves like an invocation of this function:
///
/// ```ignore
/// fn try_transmute_mut<Src, Dst>(src: &mut Src) -> Result<&mut Dst, ValidityError<&mut Src, Dst>>
/// where
///     Src: FromBytes + IntoBytes + ?Sized,
///     Dst: TryFromBytes + IntoBytes + ?Sized,
///     align_of::<Src>() >= align_of::<Dst>(),
///     size_compatible::<Src, Dst>(),
/// {
/// # /*
///     ...
/// # */
/// }
/// ```
///
/// The types `Src` and `Dst` are inferred from the calling context; they cannot
/// be explicitly specified in the macro invocation.
///
/// [`TryFromBytes`]: crate::TryFromBytes
///
/// # Size compatibility
///
/// `try_transmute_mut!` supports transmuting between `Sized` types, between
/// unsized (i.e., `?Sized`) types, and from a `Sized` type to an unsized type.
/// It supports any transmutation that preserves the number of bytes of the
/// referent, even if doing so requires updating the metadata stored in an
/// unsized "fat" reference:
///
/// ```
/// # use zerocopy::try_transmute_mut;
/// # use core::mem::size_of_val; // Not in the prelude on our MSRV
/// let src: &mut [[u8; 2]] = &mut [[0, 1], [2, 3]][..];
/// let dst: &mut [u8] = try_transmute_mut!(src).unwrap();
///
/// assert_eq!(dst.len(), 4);
/// assert_eq!(dst, [0, 1, 2, 3]);
/// let dst_size = size_of_val(dst);
/// assert_eq!(src.len(), 2);
/// assert_eq!(size_of_val(src), dst_size);
/// ```
///
/// # Examples
///
/// Transmuting between `Sized` types:
///
/// ```
/// # use zerocopy::*;
/// // 0u8 → bool = false
/// let src = &mut 0u8;
/// assert_eq!(try_transmute_mut!(src), Ok(&mut false));
///
/// // 1u8 → bool = true
/// let src = &mut 1u8;
///  assert_eq!(try_transmute_mut!(src), Ok(&mut true));
///
/// // 2u8 → bool = error
/// let src = &mut 2u8;
/// assert!(matches!(
///     try_transmute_mut!(src),
///     Result::<&mut bool, _>::Err(ValidityError { .. })
/// ));
/// ```
///
/// Transmuting between unsized types:
///
/// ```
/// # use {zerocopy::*, zerocopy_derive::*};
/// # type u16 = zerocopy::byteorder::native_endian::U16;
/// # type u32 = zerocopy::byteorder::native_endian::U32;
/// #[derive(KnownLayout, FromBytes, IntoBytes, Immutable)]
/// #[repr(C)]
/// struct SliceDst<T, U> {
///     t: T,
///     u: [U],
/// }
///
/// type Src = SliceDst<u32, u16>;
/// type Dst = SliceDst<u16, bool>;
///
/// let mut bytes = [0, 1, 0, 1, 0, 1, 0, 1];
/// let src = Src::mut_from_bytes(&mut bytes).unwrap();
///
/// assert_eq!(src.t.as_bytes(), [0, 1, 0, 1]);
/// assert_eq!(src.u.len(), 2);
/// assert_eq!(src.u.as_bytes(), [0, 1, 0, 1]);
///
/// let dst: &Dst = try_transmute_mut!(src).unwrap();
///
/// assert_eq!(dst.t.as_bytes(), [0, 1]);
/// assert_eq!(dst.u, [false, true, false, true, false, true]);
/// ```
#[macro_export]
macro_rules! try_transmute_mut {
    ($e:expr) => {{
        // Ensure that the source type is a mutable reference.
        let e: &mut _ = $e;

        #[allow(unused_imports)]
        use $crate::util::macro_util::TryTransmuteMutDst as _;
        let t = $crate::util::macro_util::Wrap::new(e);
        if false {
            // This branch exists solely to force the compiler to infer the type
            // of `Dst` *before* it attempts to resolve the method call to
            // `try_transmute_mut` in the `else` branch.
            //
            // Without this, if `Src` is `Sized` but `Dst` is `!Sized`, the
            // compiler will eagerly select the inherent impl of
            // `try_transmute_mut` (which requires `Dst: Sized`) because
            // inherent methods take priority over trait methods. It does this
            // before it realizes `Dst` is `!Sized`, leading to a compile error
            // when it checks the bounds later.
            //
            // By calling this helper (which returns `&Dst`), we force `Dst`
            // to be fully resolved. By the time it gets to the `else`
            // branch, the compiler knows `Dst` is `!Sized`, properly
            // disqualifies the inherent method, and falls back to the trait
            // implementation.
            Ok(t.transmute_mut_inference_helper())
        } else {
            t.try_transmute_mut()
        }
    }}
}

/// Includes a file and safely transmutes it to a value of an arbitrary type.
///
/// The file will be included as a byte array, `[u8; N]`, which will be
/// transmuted to another type, `T`. `T` is inferred from the calling context,
/// and must implement [`FromBytes`].
///
/// The file is located relative to the current file (similarly to how modules
/// are found). The provided path is interpreted in a platform-specific way at
/// compile time. So, for instance, an invocation with a Windows path containing
/// backslashes `\` would not compile correctly on Unix.
///
/// `include_value!` is ignorant of byte order. For byte order-aware types, see
/// the [`byteorder`] module.
///
/// [`FromBytes`]: crate::FromBytes
/// [`byteorder`]: crate::byteorder
///
/// # Examples
///
/// Assume there are two files in the same directory with the following
/// contents:
///
/// File `data` (no trailing newline):
///
/// ```text
/// abcd
/// ```
///
/// File `main.rs`:
///
/// ```rust
/// use zerocopy::include_value;
/// # macro_rules! include_value {
/// # ($file:expr) => { zerocopy::include_value!(concat!("../../../../../test/foundation/zerocopy/testdata/include_value/", $file)) };
/// # }
///
/// fn main() {
///     let as_u32: u32 = include_value!("data");
///     assert_eq!(as_u32, u32::from_ne_bytes([b'a', b'b', b'c', b'd']));
///     let as_i32: i32 = include_value!("data");
///     assert_eq!(as_i32, i32::from_ne_bytes([b'a', b'b', b'c', b'd']));
/// }
/// ```
///
/// # Use in `const` contexts
///
/// This macro can be invoked in `const` contexts.
#[doc(alias("include_bytes", "include_data", "include_type"))]
#[macro_export]
macro_rules! include_value {
    ($file:expr $(,)?) => {
        $crate::transmute!(*::core::include_bytes!($file))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! cryptocorrosion_derive_traits {
    (
        #[repr($repr:ident)]
        $(#[$attr:meta])*
        $vis:vis struct $name:ident $(<$($tyvar:ident),*>)?
        $(
            (
                $($tuple_field_vis:vis $tuple_field_ty:ty),*
            );
        )?

        $(
            {
                $($field_vis:vis $field_name:ident: $field_ty:ty,)*
            }
        )?
    ) => {
        $crate::cryptocorrosion_derive_traits!(@assert_allowed_struct_repr #[repr($repr)]);

        $(#[$attr])*
        #[repr($repr)]
        $vis struct $name $(<$($tyvar),*>)?
        $(
            (
                $($tuple_field_vis $tuple_field_ty),*
            );
        )?

        $(
            {
                $($field_vis $field_name: $field_ty,)*
            }
        )?

        // SAFETY: See inline.
        unsafe impl $(<$($tyvar),*>)? $crate::TryFromBytes for $name$(<$($tyvar),*>)?
        where
            $(
                $($tuple_field_ty: $crate::FromBytes,)*
            )?

            $(
                $($field_ty: $crate::FromBytes,)*
            )?
        {
            #[inline(always)]
            fn is_safe<A>(_: $crate::Maybe<'_, Self, A>) -> bool
            where
                A: $crate::invariant::Alignment,
            {
                // SAFETY: This macro only accepts `#[repr(C)]` and
                // `#[repr(transparent)]` structs, and this `impl` block
                // requires all field types to be `FromBytes`. Thus, all
                // initialized byte sequences constitutes valid instances of
                // `Self`.
                true
            }

            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` and
        // `#[repr(transparent)]` structs, and this `impl` block requires all
        // field types to be `FromBytes`, which is a sub-trait of `FromZeros`.
        unsafe impl $(<$($tyvar),*>)? $crate::FromZeros for $name$(<$($tyvar),*>)?
        where
            $(
                $($tuple_field_ty: $crate::FromBytes,)*
            )?

            $(
                $($field_ty: $crate::FromBytes,)*
            )?
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` and
        // `#[repr(transparent)]` structs, and this `impl` block requires all
        // field types to be `FromBytes`.
        unsafe impl $(<$($tyvar),*>)? $crate::FromBytes for $name$(<$($tyvar),*>)?
        where
            $(
                $($tuple_field_ty: $crate::FromBytes,)*
            )?

            $(
                $($field_ty: $crate::FromBytes,)*
            )?
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` and
        // `#[repr(transparent)]` structs, this `impl` block requires all field
        // types to be `IntoBytes`, and a padding check is used to ensures that
        // there are no padding bytes.
        unsafe impl $(<$($tyvar),*>)? $crate::IntoBytes for $name$(<$($tyvar),*>)?
        where
            $(
                $($tuple_field_ty: $crate::IntoBytes,)*
            )?

            $(
                $($field_ty: $crate::IntoBytes,)*
            )?

            (): $crate::util::macro_util::PaddingFree<
                Self,
                {
                    $crate::cryptocorrosion_derive_traits!(
                        @struct_padding_check #[repr($repr)]
                        $(($($tuple_field_ty),*))?
                        $({$($field_ty),*})?
                    )
                },
            >,
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` and
        // `#[repr(transparent)]` structs, and this `impl` block requires all
        // field types to be `Immutable`.
        unsafe impl $(<$($tyvar),*>)? $crate::Immutable for $name$(<$($tyvar),*>)?
        where
            $(
                $($tuple_field_ty: $crate::Immutable,)*
            )?

            $(
                $($field_ty: $crate::Immutable,)*
            )?
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }
    };
    (@assert_allowed_struct_repr #[repr(transparent)]) => {};
    (@assert_allowed_struct_repr #[repr(C)]) => {};
    (@assert_allowed_struct_repr #[$_attr:meta]) => {
        compile_error!("repr must be `#[repr(transparent)]` or `#[repr(C)]`");
    };
    (
        @struct_padding_check #[repr(transparent)]
        $(($($tuple_field_ty:ty),*))?
        $({$($field_ty:ty),*})?
    ) => {
        // SAFETY: `#[repr(transparent)]` structs cannot have the same layout as
        // their single non-zero-sized field, and so cannot have any padding
        // outside of that field.
        0
    };
    (
        @struct_padding_check #[repr(C)]
        $(($($tuple_field_ty:ty),*))?
        $({$($field_ty:ty),*})?
    ) => {
        $crate::struct_padding!(
            Self,
            None,
            None,
            [
                $($($tuple_field_ty),*)?
                $($($field_ty),*)?
            ]
        )
    };
    (
        #[repr(C)]
        $(#[$attr:meta])*
        $vis:vis union $name:ident {
            $(
                $field_name:ident: $field_ty:ty,
            )*
        }
    ) => {
        $(#[$attr])*
        #[repr(C)]
        $vis union $name {
            $(
                $field_name: $field_ty,
            )*
        }

        // SAFETY: See inline.
        unsafe impl $crate::TryFromBytes for $name
        where
            $(
                $field_ty: $crate::FromBytes,
            )*
        {
            #[inline(always)]
            fn is_safe<A>(_: $crate::Maybe<'_, Self, A>) -> bool
            where
                A: $crate::invariant::Alignment,
            {
                // SAFETY: This macro only accepts `#[repr(C)]` unions, and this
                // `impl` block requires all field types to be `FromBytes`.
                // Thus, all initialized byte sequences constitutes valid
                // instances of `Self`.
                true
            }

            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` unions, and this `impl`
        // block requires all field types to be `FromBytes`, which is a
        // sub-trait of `FromZeros`.
        unsafe impl $crate::FromZeros for $name
        where
            $(
                $field_ty: $crate::FromBytes,
            )*
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` unions, and this `impl`
        // block requires all field types to be `FromBytes`.
        unsafe impl $crate::FromBytes for $name
        where
            $(
                $field_ty: $crate::FromBytes,
            )*
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` unions, this `impl`
        // block requires all field types to be `IntoBytes`, and a padding check
        // is used to ensures that there are no padding bytes before or after
        // any field.
        unsafe impl $crate::IntoBytes for $name
        where
            $(
                $field_ty: $crate::IntoBytes,
            )*
            (): $crate::util::macro_util::PaddingFree<
                Self,
                {
                    $crate::union_padding!(
                        Self,
                        None::<usize>,
                        None::<usize>,
                        [$($field_ty),*]
                    )
                },
            >,
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        // SAFETY: This macro only accepts `#[repr(C)]` unions, and this `impl`
        // block requires all field types to be `Immutable`.
        unsafe impl $crate::Immutable for $name
        where
            $(
                $field_ty: $crate::Immutable,
            )*
        {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }
    };
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/zerocopy/unit/src/macros/tests_1256.rs"));
