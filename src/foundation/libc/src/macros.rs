/// A macro for defining #[cfg] if-else statements.
///
/// This is similar to the `if/elif` C preprocessor macro by allowing definition of a cascade of
/// `#[cfg]` cases, emitting the implementation which matches first.
///
/// This allows you to conveniently provide a long list #[cfg]'d blocks of code without having to
/// rewrite each clause multiple times.
macro_rules! cfg_if {
    // match if/else chains with a final `else`
    ($(
        if #[cfg($($meta:meta),*)] { $($it:item)* }
    ) else * else {
        $($it2:item)*
    }) => {
        cfg_if! {
            @__items
            () ;
            $( ( ($($meta),*) ($($it)*) ), )*
            ( () ($($it2)*) ),
        }
    };

    // match if/else chains lacking a final `else`
    (
        if #[cfg($($i_met:meta),*)] { $($i_it:item)* }
        $(
            else if #[cfg($($e_met:meta),*)] { $($e_it:item)* }
        )*
    ) => {
        cfg_if! {
            @__items
            () ;
            ( ($($i_met),*) ($($i_it)*) ),
            $( ( ($($e_met),*) ($($e_it)*) ), )*
            ( () () ),
        }
    };

    // Internal and recursive macro to emit all the items
    //
    // Collects all the negated `cfg`s in a list at the beginning and after the
    // semicolon is all the remaining items
    (@__items ($($not:meta,)*) ; ) => {};
    (@__items ($($not:meta,)*) ; ( ($($m:meta),*) ($($it:item)*) ),
     $($rest:tt)*) => {
        // Emit all items within one block, applying an appropriate #[cfg]. The
        // #[cfg] will require all `$m` matchers specified and must also negate
        // all previous matchers.
        cfg_if! { @__apply cfg(all($($m,)* not(any($($not),*)))), $($it)* }

        // Recurse to emit all other items in `$rest`, and when we do so add all
        // our `$m` matchers to the list of `$not` matchers as future emissions
        // will have to negate everything we just matched as well.
        cfg_if! { @__items ($($not,)* $($m,)*) ; $($rest)* }
    };

    // Internal macro to Apply a cfg attribute to a list of items
    (@__apply $m:meta, $($it:item)*) => {
        $(#[$m] $it)*
    };
}

/// Create an internal crate prelude with `core` reexports and common types.
macro_rules! prelude {
    () => {
        mod types;

        /// Frequently-used types that are available on all platforms
        ///
        /// We need to reexport the core types so this works with `rust-dep-of-std`.
        mod prelude {
            // Exports from `core`
            #[allow(unused_imports)]
            pub(crate) use core::clone::Clone;
            #[allow(unused_imports)]
            pub(crate) use core::cmp::{
                Eq,
                PartialEq,
            };
            #[allow(unused_imports)]
            pub(crate) use core::default::Default;
            #[allow(unused_imports)]
            pub(crate) use core::iter::Iterator;
            #[allow(unused_imports)]
            pub(crate) use core::marker::{
                Copy,
                Send,
                Sync,
            };
            #[allow(unused_imports)]
            pub(crate) use core::option::Option::{
                self,
                None,
                Some,
            };
            #[allow(unused_imports)]
            pub(crate) use core::prelude::v1::derive;
            #[allow(unused_imports)]
            pub(crate) use core::{
                assert,
                cfg,
                compile_error,
                debug_assert,
                fmt,
                hash,
                iter,
                mem,
                panic,
                ptr,
                unimplemented,
            };

            #[allow(unused_imports)]
            pub(crate) use fmt::Debug;
            #[allow(unused_imports)]
            pub(crate) use mem::{
                align_of,
                align_of_val,
                size_of,
                size_of_val,
            };

            #[allow(unused_imports)]
            #[cfg(any(target_os = "linux", target_os = "android", target_os = "l4re"))]
            pub(crate) use crate::types::u32_cast_ioctl;
            #[allow(unused_imports)]
            pub(crate) use crate::types::{
                cstr,
                replace_array_items,
                u16_cast_short,
                u32_cast_int,
                u32_cast_long,
                u8_slice_cast_char_slice,
                ulong_cast_int,
                ulong_cast_uint,
                CEnumRepr,
                Padding,
            };
            // Commonly used types defined in this crate
            #[allow(unused_imports)]
            pub(crate) use crate::{
                c_char,
                c_double,
                c_float,
                c_int,
                c_long,
                c_longlong,
                c_short,
                c_uchar,
                c_uint,
                c_ulong,
                c_ulonglong,
                c_ushort,
                c_void,
                intptr_t,
                size_t,
                ssize_t,
                uintptr_t,
            };
        }
    };
}

/// Implement `Clone`, `Copy`, and `Debug` for one or more structs, as well as `PartialEq`, `Eq`,
/// and `Hash` if the `extra_traits` feature is enabled.
///
/// Also mark the type with `repr(C)`.
///
/// Use [`s_no_extra_traits`] for structs where the `extra_traits` feature does not make sense, and
/// for unions.
macro_rules! s {
    ($(
        $(#[$attr:meta])*
        $pub:vis $t:ident $i:ident { $($field:tt)* }
    )*) => ($(
        s!(it: $(#[$attr])* $pub $t $i { $($field)* });
    )*);

    (it: $(#[$attr:meta])* $pub:vis union $i:ident { $($field:tt)* }) => (
        compile_error!("unions cannot derive extra traits, use s_no_extra_traits instead");
    );

    (it: $(#[$attr:meta])* $pub:vis struct $i:ident { $($field:tt)* }) => (
        #[repr(C)]
        #[::core::prelude::v1::derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
        )]
        #[cfg_attr(
            feature = "extra_traits",
            ::core::prelude::v1::derive(
                ::core::cmp::PartialEq,
                ::core::cmp::Eq,
                ::core::hash::Hash,
            )
        )]
        #[allow(deprecated)]
        $(#[$attr])*
        $pub struct $i { $($field)* }
    );
}

/// Implement `Clone`, `Copy`, and `Debug` for a tuple struct, as well as `PartialEq`, `Eq`, and
/// `Hash` if the `extra_traits` feature is enabled.
///
/// Unlike `s!`, this does *not* mark the type with `repr(C)`. Users should provide their own
/// `repr` attribute via `$attr` as necessary.
macro_rules! s_paren {
    ($(
        $(#[$attr:meta])*
        $pub:vis struct $i:ident ( $($field:tt)* );
    )*) => ($(
        #[::core::prelude::v1::derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
        )]
        #[cfg_attr(
            feature = "extra_traits",
            ::core::prelude::v1::derive(
                ::core::cmp::PartialEq,
                ::core::cmp::Eq,
                ::core::hash::Hash,
            )
        )]
        $(#[$attr])*
        $pub struct $i ( $($field)* );
    )*);
}

/// Implement `Clone`, `Copy`, and `Debug` for one or more structs/unions, but exclude `PartialEq`,
/// `Eq`, and `Hash`.
///
/// Also mark the type with `repr(C)`.
///
/// Most structs will prefer to use [`s`].
macro_rules! s_no_extra_traits {
    ($(
        $(#$attr:tt)*
        $pub:vis $t:ident $i:ident { $($field:tt)* }
    )*) => ($(
        s_no_extra_traits!(it: $(#$attr)* $pub $t $i { $($field)* });
    )*);

    (it: $(#$attr:tt)* $pub:vis union $i:ident { $($field:tt)* }) => (
        union_with_debug! {
            $(#$attr)* $pub union $i { $($field)* }
        }
    );

    (it: $(#$attr:tt)* $pub:vis struct $i:ident { $($field:tt)* }) => (
        #[repr(C)]
        #[::core::prelude::v1::derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
        )]
        $(#$attr)*
        $pub struct $i { $($field)* }
    );
}

/// Like [`s`], but (1) generates a `Default` impl for every struct in the block, and (2) adds a
/// private field to the struct to replicate the effects of the `non_exhaustive` attribute while
/// rust-lang/rust#132699 gets sorted out.
///
/// To opt out of having the private field added, annotate the struct with an `exhaustive`
/// attribute, as in:
///
/// ```ignore
/// s2! {
///     #[exhaustive]
///     struct Something {
///         ...
///     }
/// }
/// ```
///
/// See [`custom_struct`] for details.
macro_rules! s2 {
    ($(
        $(#$attr:tt)*
        $pub:vis $t:ident $i:ident { $($field:tt)* }
    )*) => ($(
        s2!(it: $(#$attr)* $pub $t $i { $($field)* });
    )*);

    (it: $(#$attr:tt)* $pub:vis union $i:ident { $($field:tt)* }) => (
        compile_error!(
            "unions cannot derive extra traits, use `s_no_extra_traits2` \
             instead"
        );
    );

    (it: $(#$attr:tt)* $pub:vis struct $i:ident { $($field:tt)* }) => (
        custom_struct! {
            attrs: {
                #[repr(C)]
                #[::core::prelude::v1::derive(
                    ::core::clone::Clone,
                    ::core::marker::Copy,
                    ::core::fmt::Debug,
                )]
                #[cfg_attr(
                    feature = "extra_traits",
                    ::core::prelude::v1::derive(
                        ::core::cmp::PartialEq,
                        ::core::cmp::Eq,
                        ::core::hash::Hash,
                    )
                )]
                #[allow(deprecated)]
            }
            $(#$attr)* $pub struct $i { $($field)* }
        }
    );
}

/// Like [`s_no_extra_traits`], but (1) generates a `Default` impl for every struct in the block,
/// and (2) adds a private field to replicate the effects of the built-in `non_exhaustive` attribute
/// while rust-lang/rust#132699 gets sorted out.
///
/// Unions are emitted just like `s_no_extra_traits!` does, with no `Default`.  A struct field of
/// union type supplies its own default via `#[custom_default(...)]`.
///
/// To opt out of having the private field added, annotate the struct with an `exhaustive`
/// attribute, as in:
///
/// ```ignore
/// s_no_extra_traits2! {
///     #[exhaustive]
///     struct Something {
///         ...
///     }
/// }
/// ```
macro_rules! s_no_extra_traits2 {
    ($(
        $(#$attr:tt)*
        $pub:vis $t:ident $i:ident { $($field:tt)* }
    )*) => ($(
        s_no_extra_traits2!(it: $(#$attr)* $pub $t $i { $($field)* });
    )*);

    (it: $(#$attr:tt)* $pub:vis union $i:ident { $($field:tt)* }) => (
        union_with_debug! {
            $(#$attr)* $pub union $i { $($field)* }
        }
    );

    (it: $(#$attr:tt)* $pub:vis struct $i:ident { $($field:tt)* }) => (
        custom_struct! {
            attrs: {
                #[repr(C)]
                #[::core::prelude::v1::derive(
                    ::core::clone::Clone,
                    ::core::marker::Copy,
                    ::core::fmt::Debug,
                )]
            }
            $(#$attr)* $pub struct $i { $($field)* }
        }
    );
}

/// Emit a union plus its `Debug` impl.
///
/// Unions can't derive `Debug`, so it is written out here. Attributes are split like
/// [`custom_struct`] does. Everything goes on the union, but only the `cfg`s are repeated on the
/// impl, otherwise a union that is configured out leaves an impl behind.
macro_rules! union_with_debug {
    (
        $(#$attr:tt)*
        $vis:vis union $name:ident { $($body:tt)* }
    ) => {
        union_with_debug! {
            @split_attrs,
            cfg_attrs: { },
            other_attrs: { },
            remaining_attrs: { $(#$attr)* },
            found_default_via_unsafe_zeroed: false,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // a `cfg` also has to gate the impl
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: {
            #[cfg($($cfg:tt)*)]
            $($tail:tt)*
        },
        found_default_via_unsafe_zeroed: $found_default:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        union_with_debug! {
            @split_attrs,
            cfg_attrs: { $($cfg_attrs)* #[cfg($($cfg)*)] },
            other_attrs: { $($other_attrs)* },
            remaining_attrs: { $($tail)* },
            found_default_via_unsafe_zeroed: $found_default,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // a `cfg` also has to gate the impl
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: {
            #[unsafe(union_default_via_zeroed)]
            $($tail:tt)*
        },
        found_default_via_unsafe_zeroed: $found_default:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        union_with_debug! {
            @split_attrs,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* },
            remaining_attrs: { $($tail)* },
            found_default_via_unsafe_zeroed: true,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // anything else belongs to the union only
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: {
            #$other:tt
            $($tail:tt)*
        },
        found_default_via_unsafe_zeroed: $found_default:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        union_with_debug! {
            @split_attrs,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* #$other },
            remaining_attrs: { $($tail)* },
            found_default_via_unsafe_zeroed: $found_default,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // done
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: { },
        found_default_via_unsafe_zeroed: $found_default:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        #[repr(C)]
        #[::core::prelude::v1::derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
        )]
        $($other_attrs)*
        $($cfg_attrs)*
        $vis union $name { $($body)* }

        $($cfg_attrs)*
        #[allow(deprecated)]
        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(::core::stringify!($name)).finish_non_exhaustive()
            }
        }

        emit_union_default_via_unsafe_zeroed! {
            found_default_via_unsafe_zeroed: $found_default,
            cfg_attrs: { $($cfg_attrs)* },
            name: $name,
        }
    };
}

/// Add a `Default` impl that is just `mem::zeroed`.
macro_rules! emit_union_default_via_unsafe_zeroed {
    (
        found_default_via_unsafe_zeroed: false,
        cfg_attrs: { $($cfg_attrs:tt)* },
        name: $name:ident,
    ) => {
    };

    (
        found_default_via_unsafe_zeroed: true,
        cfg_attrs: { $($cfg_attrs:tt)* },
        name: $name:ident,
    ) => {
        $($cfg_attrs)*
        impl ::core::default::Default for $name {
            fn default() -> Self {
                unsafe { ::core::mem::zeroed() }
            }
        }
    };
}

/// Emit a struct with the given derive attributes plus a generated `Default` impl. Ensure that the
/// record has an additional private field added to replicate `#[non_exhaustive]`, unless it is
/// annotated with `#[exhaustive]`.
///
/// Fields default to `Default::default()`. A field whose default can't be derived must carry
/// `#[custom_default(EXPR)]` as its *first* attribute, and `EXPR` is used instead.
///
/// This works by scanning each field for `#[custom_default]` attributes. If one exists, the
/// attribute's contents are added to `processed_field_defaults` and will be used in the expansion
/// for `Default`. If it does not exist, `Default::default()` is used instead. In either case, the
/// field is added to `processed_fields` with `#[custom_default]` stripped if necessary, and
/// `custom_struct` is invoked again with the remaining fields.
///
/// Attributes are split into `cfg_attrs` and `other_attrs` before the fields are scanned. Both go
/// on the struct, but only the `cfg`s are repeated on the `Default` impl. A `cfg` decides whether
/// the type exists at all, so without it a configured-out struct leaves an impl behind referring to
/// a type that isn't there.
///
/// Out of `other_attrs`, we scan for `#[exhaustive]`. If found, we remove it but take into account
/// that the record should be expanded _without_ an additional private field. The scan for both
/// `cfg` attributes and the (non-existent) `exhaustive` attribute is done in one linear pass.
macro_rules! custom_struct {
    // entry; `attrs` is the attribute block the caller wants on the struct
    // (repr, derives, etc.), which is merged with the struct's own attributes.
    (
        attrs: { $($attrs:tt)* }
        $(#$attr:tt)*
        $vis:vis struct $name:ident { $($body:tt)* }
    ) => {
        custom_struct! {
            @split_attrs,
            cfg_attrs: { },
            other_attrs: { },
            remaining_attrs: { $($attrs)* $(#$attr)* },
            found_exhaustive_attr: false,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // a `cfg` also has to gate the impl
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: {
            #[cfg($($cfg:tt)*)]
            $($tail:tt)*
        },
        found_exhaustive_attr: $found_exhaustive:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        custom_struct! {
            @split_attrs,
            cfg_attrs: { $($cfg_attrs)* #[cfg($($cfg)*)] },
            other_attrs: { $($other_attrs)* },
            remaining_attrs: { $($tail)* },
            found_exhaustive_attr: $found_exhaustive,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // `exhaustive` must be taken into account as many times as it appears,
    // though the effect is the same with a single annotation.
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: {
            #[exhaustive]
            $($tail:tt)*
        },
        found_exhaustive_attr: $_:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        custom_struct! {
            @split_attrs,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* },
            remaining_attrs: { $($tail)* },
            found_exhaustive_attr: true,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // anything else belongs to the struct only
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: {
            #$other:tt
            $($tail:tt)*
        },
        found_exhaustive_attr: $found_exhaustive:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        custom_struct! {
            @split_attrs,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* #$other },
            remaining_attrs: { $($tail)* },
            found_exhaustive_attr: $found_exhaustive,
            vis: $vis,
            name: $name,
            body: { $($body)* },
        }
    };

    // attributes are split, move on to the fields
    (
        @split_attrs,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        remaining_attrs: { },
        found_exhaustive_attr: $found_exhaustive:tt,
        vis: $vis:vis,
        name: $name:ident,
        body: { $($body:tt)* },
    ) => {
        custom_struct! {
            @struct,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* },
            found_exhaustive_attr: $found_exhaustive,
            vis: $vis,
            name: $name,
            processed_fields: { },
            processed_field_defaults: { },
            remaining_fields: { $($body)* },
        }
    };

    // field led by #[custom_default(...)]
    (
        @struct,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        found_exhaustive_attr: $found_exhaustive:tt,
        vis: $vis:vis,
        name: $name:ident,
        processed_fields: { $($processed_fields:tt)* },
        processed_field_defaults: { $($processed_field_defaults:tt)* },
        remaining_fields: {
            #[custom_default($default:expr)]
            $(#[$fattr:meta])*
            $fvis:vis $fname:ident: $fty:ty,
            $($tail:tt)*
        },
    ) => {
        custom_struct! {
            @struct,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* },
            found_exhaustive_attr: $found_exhaustive,
            vis: $vis,
            name: $name,
            processed_fields: { $($processed_fields)* $(#[$fattr])* $fvis $fname: $fty, },
            processed_field_defaults: {
                $($processed_field_defaults)*
                $(#[$fattr])* $fname: $default,
            },
            remaining_fields: { $($tail)* },
        }
    };

    // plain field
    (
        @struct,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        found_exhaustive_attr: $found_exhaustive:tt,
        vis: $vis:vis,
        name: $name:ident,
        processed_fields: { $($processed_fields:tt)* },
        processed_field_defaults: { $($processed_field_defaults:tt)* },
        remaining_fields: {
            $(#[$fattr:meta])*
            $fvis:vis $fname:ident: $fty:ty,
            $($tail:tt)*
        },
    ) => {
        custom_struct! {
            @struct,
            cfg_attrs: { $($cfg_attrs)* },
            other_attrs: { $($other_attrs)* },
            found_exhaustive_attr: $found_exhaustive,
            vis: $vis,
            name: $name,
            processed_fields: { $($processed_fields)* $(#[$fattr])* $fvis $fname: $fty, },
            processed_field_defaults: {
                $($processed_field_defaults)*
                $(#[$fattr])* $fname: ::core::default::Default::default(),
            },
            remaining_fields: { $($tail)* },
        }
    };

    // done
    (
        @struct,
        cfg_attrs: { $($cfg_attrs:tt)* },
        other_attrs: { $($other_attrs:tt)* },
        found_exhaustive_attr: $found_exhaustive:tt,
        vis: $vis:vis,
        name: $name:ident,
        processed_fields: { $($processed_fields:tt)* },
        processed_field_defaults: { $($processed_field_defaults:tt)* },
        remaining_fields: { },
    ) => {
        emit_struct_definition! {
            found_exhaustive_attr: $found_exhaustive,
            body: {
                $($other_attrs)*
                $($cfg_attrs)*
                $vis $name { $($processed_fields)* }
            }
        }

        $($cfg_attrs)*
        // The impl names the type and its fields, which warns if either is
        // deprecated.
        #[allow(deprecated)]
        impl ::core::default::Default for $name {
            // Field attributes (`#[cfg]`, doc comments) get forwarded to the
            // initializer too. Docs are harmless there but trip the lint, so
            // silence it.
            #[allow(unused_doc_comments)]
            fn default() -> Self {
                emit_struct_default_body! {
                    found_exhaustive_attr: $found_exhaustive,
                    body: { $($processed_field_defaults)* }
                }
            }
        }
    };
}

/// Expands the definition of the record defined at [`custom_struct`], with either one of an
/// additional private field or with its verbatim fields.
macro_rules! emit_struct_definition {
    (
        found_exhaustive_attr: false,
        body: { $(#[$attr:meta])* $vis:vis $name:ident { $($field:tt)* } }
    ) => {
        #[allow(clippy::manual_non_exhaustive)]
        $(#[$attr])*
        $vis struct $name { $($field)* __non_exhaustive: () }
    };

    (
        found_exhaustive_attr: true,
        body: { $(#[$attr:meta])* $vis:vis $name:ident { $($field:tt)* } }
    ) => {
        $(#[$attr])*
        $vis struct $name { $($field)* }
    };
}

/// Expands the `Default` implementation of the record defined at [`custom_struct`], with either one
/// of an additional private field initialized to the unit value, or the record's fields verbatim.
macro_rules! emit_struct_default_body {
    (
        found_exhaustive_attr: false,
        body: { $($field_default:tt)* }
    ) => {
        Self { $($field_default)* __non_exhaustive: () }
    };

    (
        found_exhaustive_attr: true,
        body: { $($field_default:tt)* }
    ) => {
        Self { $($field_default)* }
    };
}

/// Create an uninhabited type that can't be constructed. It implements `Debug`, `Clone`, and
/// `Copy`, but these aren't meaningful for extern types so they should eventually be removed.
///
/// Really what we want here is something that also can't be named without indirection (in ADTs or
/// function signatures), but this doesn't exist.
macro_rules! extern_ty {
    ($(
        $(#[$attr:meta])*
        $vis:vis type $i:ident;
    )*) => ($(
        $(#[$attr])*
        /// This is an extern type ("opaque" or "incomplete" type in C).
        ///
        /// <div class="warning">
        /// This type's current representation allows inspecting some properties, such as via
        /// <code>size_of</code>, and it is technically possible to construct the type within
        /// <code>MaybeUninit</code>, However, this <strong>MUST NOT</strong> be relied upon
        /// because a future version of <code>libc</code> may switch to a proper
        /// <a href="https://rust-lang.github.io/rfcs/1861-extern-types.html">extern type</a>
        /// representation when available.
        /// </div>
        // ^ unfortunately warning blocks currently don't render markdown so we need to
        // use raw HTML.
        //
        // Representation based on the Nomicon:
        // <https://doc.rust-lang.org/nomicon/ffi.html#representing-opaque-structs>.
        //
        // FIXME(1.0): These traits are unreachable and should be removed.
        #[::core::prelude::v1::derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
        )]
        #[repr(C)]
        $vis struct $i {
            _data: (),
            _marker: ::core::marker::PhantomData<(*mut u8, ::core::marker::PhantomPinned)>,
        }
    )*);
}

/// Implement `Clone` and `Copy` for an enum, as well as `Debug`, `Eq`, `Hash`, and
/// `PartialEq` if the `extra_traits` feature is enabled.
// FIXME(#4419): Replace all uses of `e!` with `c_enum!`
macro_rules! e {
    ($(
        $(#[$attr:meta])*
        pub enum $i:ident { $($field:tt)* }
    )*) => ($(
        #[cfg_attr(
            feature = "extra_traits",
            ::core::prelude::v1::derive(Eq, Hash, PartialEq)
        )]
        #[::core::prelude::v1::derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
        )]
        $(#[$attr])*
        pub enum $i { $($field)* }
    )*);
}

/// Represent a C enum as Rust constants and a type.
///
/// C enums can't soundly be mapped to Rust enums since C enums are allowed to have duplicates or
/// unlisted values, but this is UB in Rust. This enum doesn't implement any traits, its main
/// purpose is to calculate the correct enum values.
///
/// Use the magic name `#anon` if the C enum doesn't create a type.
///
/// See <https://github.com/rust-lang/libc/issues/4419> for more.
macro_rules! c_enum {
    // Matcher for multiple enums
    ($(
        $(#[repr($repr:ty)])?
        $vis:vis enum $($ty_name:ident)? $(#$anon:ident)? {
            $(
                $(#[$meta:meta])*
                $field_vis:vis $variant:ident $(= $value:expr)?,
            )+
        }
    )+) => {
        $(c_enum!(@single;
            $(#[repr($repr)])?
            $vis enum $($ty_name)? $(#$anon)? {
                $(
                    $(#[$meta])*
                    $field_vis $variant $(= $value)?,
                )+
            }
        );)+
    };

    // Matcher for a single enum
    (@single;
        $(#[repr($repr:ty)])?
        $vis:vis enum $ty_name:ident {
            $(
                $(#[$meta:meta])*
                $field_vis:vis $variant:ident $(= $value:expr)?,
            )+
        }
    ) => {
        $vis type $ty_name = c_enum!(@ty $($repr)?);
        c_enum! {
            @variant;
            ty: $ty_name;
            default: 0;
            variants: [$(
                $(#[$meta])*
                $field_vis $variant $(= $value)?,
            )+]
        }
    };

    // Matcher for a single anonymous enum
    (@single;
        $(#[repr($repr:ty)])?
        $vis:vis enum #anon {
            $(
                $(#[$meta:meta])*
                $field_vis:vis $variant:ident $(= $value:expr)?,
            )+
        }
    ) => {
        c_enum! {
            @variant;
            ty: c_enum!(@ty $($repr)?);
            default: 0;
            variants: [$(
                $(#[$meta])*
                $field_vis $variant $(= $value)?,
            )+]
        }
    };

    // Matcher for variants: eats a single variant then recurses with the rest
    (@variant;
        ty: $_ty_name:ty;
        default: $_idx:expr;
        variants: []
    ) => { /* end of the chain */ };
    (
        @variant;
        ty: $ty_name:ty;
        default: $default_val:expr;
        variants: [
            $(#[$meta:meta])*
            $field_vis:vis $variant:ident $(= $value:expr)?,
            $($tail:tt)*
        ]
    ) => {
        $(#[$meta])*
        #[allow(deprecated)]
        $field_vis const $variant: $ty_name = {
            #[allow(unused_variables)]
            let r = $default_val;
            $(let r = $value;)?
            r
        };

        // The next value is always one more than the previous value, unless
        // set explicitly.
        c_enum! {
            @variant;
            ty: $ty_name;
            default: $variant + 1;
            variants: [$($tail)*]
        }
    };

    // Use a specific type if provided, otherwise default to `CEnumRepr`
    (@ty $repr:ty) => { $repr };
    (@ty) => { $crate::prelude::CEnumRepr };
}

/// Define a function that can be either `safe` or `unsafe` and optionally `const`. This always
/// marks the function inline and adds `extern "C"`.
macro_rules! f {
    ($(
        $(#[$attr:meta])*
        pub $(const $($const_dummy:literal)?)?
        $(unsafe $($unsafe_dummy:literal)?)? $(safe $($safe_dummy:literal)?)?
        fn $i:ident ($($arg:ident: $argty:ty),* $(,)?) -> $ret:ty
            $body:block
    )+) => {$(
        f! {
            @single
            $(#[$attr])*
            pub $(const $($const_dummy)?)?
            $(unsafe $($unsafe_dummy)?)? $(safe $($safe_dummy)?)?
            fn $i ($($arg: $argty),*) -> $ret
                $body
        }
    )+};

    (@single
        $(#[$attr:meta])*
        pub $(const $($const_dummy:literal)?)? unsafe
        fn $i:ident ($($arg:ident: $argty:ty),* $(,)?) -> $ret:ty
            $body:block
    ) => {
        #[inline]
        $(#[$attr])*
        pub $(const $($const_dummy)?)? unsafe extern "C"
        fn $i ($($arg: $argty),*) -> $ret
            $body
    };

    (@single
        $(#[$attr:meta])*
        pub $(const $($const_dummy:literal)?)? safe
        fn $i:ident ($($arg:ident: $argty:ty),* $(,)?) -> $ret:ty
            $body:block
    ) => {
        #[inline]
        $(#[$attr])*
        pub $(const $($const_dummy)?)? extern "C"
        fn $i ($($arg: $argty),*) -> $ret
            $body
    };

    (@single
        pub $(const $($const_dummy:literal)?)?
        fn $i:ident ($($arg:ident: $argty:ty),* $(,)?) -> $ret:ty
            $body:block
    ) => {
        compile_error!("either `safe` or `unsafe` must be specified");
    };
}

// This macro is used to deprecate items that should be accessed via the mach2 crate
macro_rules! deprecated_mach {
    (pub const $id:ident: $ty:ty = $expr:expr;) => {
        #[deprecated(
            since = "0.2.55",
            note = "Use the `mach2` crate instead",
        )]
        #[allow(deprecated)]
        pub const $id: $ty = $expr;
    };
    ($(pub const $id:ident: $ty:ty = $expr:expr;)*) => {
        $(
            deprecated_mach!(
                pub const $id: $ty = $expr;
            );
        )*
    };
    (pub type $id:ident = $ty:ty;) => {
        #[deprecated(
            since = "0.2.55",
            note = "Use the `mach2` crate instead",
        )]
        #[allow(deprecated)]
        pub type $id = $ty;
    };
    ($(pub type $id:ident = $ty:ty;)*) => {
        $(
            deprecated_mach!(
                pub type $id = $ty;
            );
        )*
    }
}

/// Polyfill for std's `offset_of`.
// FIXME(msrv): stabilized in std in 1.77
macro_rules! offset_of {
    ($Ty:path, $field:ident) => {{
        // Taken from bytemuck, avoids accidentally calling on deref
        #[allow(clippy::unneeded_wildcard_pattern)]
        let $Ty { $field: _, .. };
        let data = core::mem::MaybeUninit::<$Ty>::uninit();
        let ptr = data.as_ptr();
        // nested unsafe, see f!
        #[allow(unused_unsafe)]
        // SAFETY: computed address is inbounds since we have a stack alloc for
        // T
        let fptr = unsafe { core::ptr::addr_of!((*ptr).$field) };
        let off = (fptr as usize).checked_sub(ptr as usize).unwrap();
        core::assert!(off <= core::mem::size_of::<$Ty>());
        off
    }};
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/libc/unit/src/macros/tests_1100.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/libc/unit/src/macros/macro_checks_1499.rs"));
