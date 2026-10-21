//! [`tuple`] operations.
//!
//! # Indexing
//! `tuple` types (with up to twelve elements) allow access to values by index, because they implement the [`core::ops::Index`] and [`core::ops::IndexMut`] traits:
//!
//! ```rust
//! use std::ops::{Index, IndexMut};
//! use typenum::{U0, U1, U2, U3, Len};
//!
//! let tuple = ("hello", 5, true);
//!
//! assert_eq!(tuple[U0::new()], "hello");
//! assert_eq!(tuple[U1::new()], 5);
//! assert_eq!(tuple[U2::new()], true);
//!
//! assert_eq!(tuple.len(), U3::new());
//! ```
//!
//! Attempting to use an out-of-bounds index is a **compile-time** error.
//!
//! ```compile_fail
//! use std::ops::{Index, IndexMut};
//! use typenum::{U0, U1, U2};
//!
//! let tuple = ("hello", 5, true);
//!
//! assert_eq!(tuple[U0::new()], "hello"); // OK, the tuple has 3 elements
//! assert_eq!(tuple[U3::new()], "this is an error!");
//! ```

use crate::{Len, U0};

impl Len for () {
    type Output = U0;

    fn len(&self) -> Self::Output {
        U0::new()
    }
}

macro_rules! generate_tuple_impls {
    (
        $( $u:ident ),* ;
        $( $t:ident ),* ;
    ) => {
        generate_tuple_impls!(@__recur__
             [ ] [ $( $u )* ];
             [ ] [ $( $t )* ];
        );
    };

    (@__recur__
        [ ] [ ];
        [ ] [ ];
    ) => { /* terminal-case */ };

    (@__recur__
        [ $( $uhead:ident )* ] [ $u:ident $( $utail:ident )+ ];
        [ $( $thead:ident )* ] [ $t:ident $( $ttail:ident )+ ];
    ) => {
        generate_tuple_impls!(@__recur__
            [ $( $uhead )* $u ] [ $( $utail )* ];
            [ $( $thead )* $t ] [ $( $ttail )* ];
        );

        generate_tuple_impls!(@__expand_at__
            [ $( $uhead )* ] [ $u $( $utail )* ];
            [ $( $thead )* ] [ $t $( $ttail )* ];
        );
    };

    (@__recur__
        [ $( $uhead:ident )* ] [ $u:ident ];
        [ $( $thead:ident )* ] [ $t:ident ];
    ) => {

        impl< $( $thead, )* $t, > $crate::Len   for ( $( $thead, )* $t, ) {
            type Output = <$crate::$u as core::ops::Add<$crate::B1>>::Output;

            #[inline]
            fn len(&self) -> Self::Output {
                Self::Output::new()
            }
        }

        generate_tuple_impls!(@__recur__
            [ ] [ $( $uhead )* ];
            [ ] [ $( $thead )* ];
        );

        generate_tuple_impls!(@__expand_at__
            [ $( $uhead )* ] [ $u ];
            [ $( $thead )* ] [ $t ];
        );
    };


    (@__expand_at__
        [ $( $uhead:ident )* ] [ $u:ident $( $utail:ident )* ];
        [ $( $thead:ident )* ] [ $t:ident $( $ttail:ident )* ];
    ) => {
        impl< $( $thead, )* $t, $( $ttail ),* > core::ops::Index<$crate::$u> for ( $( $thead, )* $t, $( $ttail ),* ) {
            type Output = $t;

            fn index(&self, _index: $crate::$u) -> &Self::Output {
                #[allow(unused_variables)]
                #[allow(non_snake_case)]
                let ( $( $thead, )* $t, ..) = self;

                $t
            }
        }

        impl< $( $thead, )* $t, $( $ttail ),* > core::ops::IndexMut<$crate::$u> for ( $( $thead, )* $t, $( $ttail ),* ) {
            fn index_mut(&mut self, _index: $crate::$u) -> &mut Self::Output {
                #[allow(unused_variables)]
                #[allow(non_snake_case)]
                let ( $( $thead, )* $t, ..) = self;

                $t
            }
        }
    };
}

generate_tuple_impls! {
    U0, U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11;
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11;
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/typenum/unit/src/tuple/test_131.rs"));
