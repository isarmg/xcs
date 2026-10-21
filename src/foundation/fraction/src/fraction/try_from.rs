use ::numeric::collection::ToPrimitive;

use crate::{generic::GenericInteger, GenericFraction};

use std::convert::TryFrom;

macro_rules! generic_fraction_into_primitive {
    ( $( $(#[$cfg:meta])* fn $method:ident -> $IntoT:ident ; )*) => {$(
        $(#[$cfg])*
        impl<T> TryFrom<GenericFraction<T>> for $IntoT
        where
            T: Clone + GenericInteger,
        {
            type Error = ();

            fn try_from(value: GenericFraction<T>) -> Result<Self, Self::Error> {
                value.$method().ok_or(())
            }
        }
    )*}
}

generic_fraction_into_primitive!(
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
    use ::numeric::collection::BigInt;

    use crate::{
        generic::{read_generic_integer, GenericInteger},
        BigUint, GenericFraction, Sign,
    };
    use std::convert::TryFrom;

    impl<T> TryFrom<GenericFraction<T>> for BigUint
    where
        T: Clone + GenericInteger,
    {
        type Error = ();

        fn try_from(value: GenericFraction<T>) -> Result<Self, Self::Error> {
            match value {
                GenericFraction::NaN => Err(()),
                GenericFraction::Infinity(_) => Err(()),
                GenericFraction::Rational(Sign::Plus, ref r) if *r.denom() == T::one() => {
                    if let Some((Sign::Plus, v)) = read_generic_integer(r.numer().clone()) {
                        Ok(v)
                    } else {
                        Err(())
                    }
                }
                _ => Err(()),
            }
        }
    }

    impl<T> TryFrom<GenericFraction<T>> for BigInt
    where
        T: Clone + GenericInteger,
    {
        type Error = ();

        fn try_from(value: GenericFraction<T>) -> Result<Self, Self::Error> {
            match value {
                GenericFraction::NaN => Err(()),
                GenericFraction::Infinity(_) => Err(()),
                GenericFraction::Rational(sign, ref r) if *r.denom() == T::one() => {
                    if let Some((s, v)) = read_generic_integer(r.numer().clone()) {
                        // Numerator must always be positive, but let's do this for hygiene
                        let sign = sign * s;

                        let result = if sign.is_negative() { v * -1 } else { v };

                        Ok(result)
                    } else {
                        Err(())
                    }
                }
                _ => Err(()),
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/try_from/test_100.rs"));
