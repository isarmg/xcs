use super::GenericDecimal;
use generic::GenericInteger;
use num::ToPrimitive;
use std::convert::TryFrom;

macro_rules! generic_decimal_into_primitive {
    ( $( $(#[$cfg:meta])* fn $method:ident -> $IntoT:ident ; )*) => {$(
        $(#[$cfg])*
        impl<T, P> TryFrom<GenericDecimal<T, P>> for $IntoT
        where
            T: Clone + ToPrimitive + GenericInteger,
            P: Copy + Into<usize> + GenericInteger
        {
            type Error = ();

            fn try_from(value: GenericDecimal<T, P>) -> Result<Self, Self::Error> {
                value.$method().ok_or(())
            }
        }
    )*}
}

generic_decimal_into_primitive!(
    fn to_u8 -> u8 ;
    fn to_i8 -> i8 ;
    fn to_u16 -> u16 ;
    fn to_i16 -> i16 ;
    fn to_u32 -> u32 ;
    fn to_i32 -> i32 ;
    fn to_u64 -> u64 ;
    fn to_i64 -> i64 ;
    fn to_u128 -> u128 ;
    fn to_i128 -> i128 ;
    fn to_usize -> usize ;
    fn to_isize -> isize ;
    fn to_f32 -> f32 ;
    fn to_f64 -> f64 ;
);

#[cfg(feature = "with-bigint")]
pub mod with_bigint {
    use num::BigInt;

    use crate::{generic::GenericInteger, BigUint, GenericDecimal};
    use std::convert::TryFrom;

    impl<T, P> TryFrom<GenericDecimal<T, P>> for BigUint
    where
        T: Clone + GenericInteger,
        P: Copy + GenericInteger + Into<usize>,
    {
        type Error = ();

        fn try_from(value: GenericDecimal<T, P>) -> Result<Self, Self::Error> {
            Self::try_from(value.0)
        }
    }

    impl<T, P> TryFrom<GenericDecimal<T, P>> for BigInt
    where
        T: Clone + GenericInteger,
        P: Copy + GenericInteger + Into<usize>,
    {
        type Error = ();

        fn try_from(value: GenericDecimal<T, P>) -> Result<Self, Self::Error> {
            Self::try_from(value.0)
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/decimal/try_from/test_72.rs"));
