//! <div class="warning">
//!
//! This crate has been "replaced" by the [`cfg_select!`] macro, which is stable since Rust 1.95.0 with a slightly different syntax. Barring breakages and security fixes, this crate will no longer be updated.
//!
//! </div>
//!
//! A macro for defining `#[cfg]` if-else statements.
//!
//! The macro provided by this crate, `cfg_if`, is similar to the `if/elif` C
//! preprocessor macro by allowing definition of a cascade of `#[cfg]` cases,
//! emitting the implementation which matches first.
//!
//! This allows you to conveniently provide a long list `#[cfg]`'d blocks of code
//! without having to rewrite each clause multiple times.
//!
//! # Example
//!
//! ```
//! cfg_if::cfg_if! {
//!     if #[cfg(unix)] {
//!         fn foo() { /* unix specific functionality */ }
//!     } else if #[cfg(target_pointer_width = "32")] {
//!         fn foo() { /* non-unix, 32-bit functionality */ }
//!     } else {
//!         fn foo() { /* fallback implementation */ }
//!     }
//! }
//!
//! # fn main() {}
//! ```

#![no_std]
#![doc(html_root_url = "https://docs.rs/cfg-if")]
#![deny(missing_docs)]
#![cfg_attr(test, allow(unexpected_cfgs))] // we test with features that do not exist

/// The main macro provided by this crate. See crate documentation for more
/// information.
#[macro_export]
macro_rules! cfg_if {
    (
        if #[cfg( $($i_meta:tt)+ )] { $( $i_tokens:tt )* }
        $(
            else if #[cfg( $($ei_meta:tt)+ )] { $( $ei_tokens:tt )* }
        )*
        $(
            else { $( $e_tokens:tt )* }
        )?
    ) => {
        $crate::cfg_if! {
            @__items () ;
            (( $($i_meta)+ ) ( $( $i_tokens )* )),
            $(
                (( $($ei_meta)+ ) ( $( $ei_tokens )* )),
            )*
            $(
                (() ( $( $e_tokens )* )),
            )?
        }
    };

    // Internal and recursive macro to emit all the items
    //
    // Collects all the previous cfgs in a list at the beginning, so they can be
    // negated. After the semicolon are all the remaining items.
    (@__items ( $( ($($_:tt)*) , )* ) ; ) => {};
    (
        @__items ( $( ($($no:tt)+) , )* ) ;
        (( $( $($yes:tt)+ )? ) ( $( $tokens:tt )* )),
        $( $rest:tt , )*
    ) => {
        // Emit all items within one block, applying an appropriate #[cfg]. The
        // #[cfg] will require all `$yes` matchers specified and must also negate
        // all previous matchers.
        #[cfg(all(
            $( $($yes)+ , )?
            not(any( $( $($no)+ ),* ))
        ))]
        // Subtle: You might think we could put `$( $tokens )*` here. But if
        // that contains multiple items then the `#[cfg(all(..))]` above would
        // only apply to the first one. By wrapping `$( $tokens )*` in this
        // macro call, we temporarily group the items into a single thing (the
        // macro call) that will be included/excluded by the `#[cfg(all(..))]`
        // as appropriate. If the `#[cfg(all(..))]` succeeds, the macro call
        // will be included, and then evaluated, producing `$( $tokens )*`. See
        // also the "issue #90" test below.
        $crate::cfg_if! { @__temp_group $( $tokens )* }

        // Recurse to emit all other items in `$rest`, and when we do so add all
        // our `$yes` matchers to the list of `$no` matchers as future emissions
        // will have to negate everything we just matched as well.
        $crate::cfg_if! {
            @__items ( $( ($($no)+) , )* $( ($($yes)+) , )? ) ;
            $( $rest , )*
        }
    };

    // See the "Subtle" comment above.
    (@__temp_group $( $tokens:tt )* ) => {
        $( $tokens )*
    };
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/cfg-if/unit/src/lib/tests_104.rs"));
