//! Optimistic type conversion
//!
//! When [std::convert::TryFrom] and [std::convert::TryInto] get stabilised
//! they should be used instead. However, at this point we need our
//! own implementation for optional conversion between types.
//!
//! The particular use case is when we have a data type claiming
//! more space than it's using and we want to move the value into
//! another, smaller data type. Such operation may only be performed
//! at runtime with particular checks that the value fits into the
//! smaller data type. Otherwise we cannot perform safe cast.
//!
//! # Examples
//! Integer conversion
//! ```
//! use fraction::convert::TryToConvertFrom;
//!
//! assert_eq!(Some(255u8), u8::try_to_convert_from(255u16));
//! assert_eq!(None, u8::try_to_convert_from(256u16));
//! ```
//!
//! Fraction conversion
//! ```
//! use fraction::{GenericFraction, convert::TryToConvertFrom, One};
//!
//! type F8 = GenericFraction<u8>;
//! type F16 = GenericFraction<u16>;
//!
//! let f8_255 = F8::new(255u8, 1u8);
//! let f16_255 = F16::try_to_convert_from(f8_255).unwrap();
//! let f16_256 = f16_255 + F16::one();
//!
//! assert_eq!(Some(f8_255), F8::try_to_convert_from(f16_255));
//! assert_eq!(None, F8::try_to_convert_from(f16_256));
//! ```

use generic::{read_generic_integer, GenericInteger};
use num::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Integer, One};

use super::GenericFraction;

#[cfg(feature = "with-bigint")]
use super::{BigInt, BigUint};

pub trait TryToConvertFrom<F>: Sized {
    fn try_to_convert_from(src: F) -> Option<Self>;
}

macro_rules! convert_impl {
    (unsigned; $($T:ty),+) => {
        $(
            impl<T> TryToConvertFrom<T> for $T
            where
                T: GenericInteger + Clone + One + CheckedAdd + CheckedDiv + CheckedMul + CheckedSub + PartialOrd,
                $T: GenericInteger
            {
                fn try_to_convert_from(src: T) -> Option<Self> {
                    read_generic_integer::<Self, T>(src)
                        .map_or_else(
                            || None,
                            |(sign, val)| if sign.is_positive () { Some(val) } else { None }
                        )
                }
            }
        )+
    };

    (signed; $($T:ty),+) => {
        $(
            impl<T> TryToConvertFrom<T> for $T
            where
                T: GenericInteger + Clone + One + CheckedAdd + CheckedDiv + CheckedMul + CheckedSub + PartialOrd,
                $T: GenericInteger
            {
                fn try_to_convert_from(src: T) -> Option<Self> {
                    read_generic_integer::<Self, T>(src)
                        .map_or_else(
                            || None,
                            |(sign, val)| Some(if sign.is_negative() {-val} else {val})
                        )
                }
            }
        )+
    };

    /* when Rust gets specializations we'll be able to do cheaper conversions for some types
    (cast; $($F:ty => $T:ty),+) => {
        $(
            impl TryToConvertFrom<$F> for $T
            {
                fn try_to_convert_from(src: $F) -> Option<$T> {
                    Some(src as $T)
                }
            }
        )+
    };
    */
}

convert_impl!(unsigned; u8, u16, u32, u64, u128, usize);
convert_impl!(signed; i8, i16, i32, i64, i128, isize);

#[cfg(feature = "with-bigint")]
convert_impl!(unsigned; BigUint);

#[cfg(feature = "with-bigint")]
convert_impl!(signed; BigInt);

impl<T, F> TryToConvertFrom<GenericFraction<F>> for GenericFraction<T>
where
    T: TryToConvertFrom<F> + Clone + Integer,
    F: Clone + Integer,
{
    fn try_to_convert_from(src: GenericFraction<F>) -> Option<Self> {
        match src {
            GenericFraction::NaN => Some(GenericFraction::NaN),
            GenericFraction::Infinity(sign) => Some(GenericFraction::Infinity(sign)),
            GenericFraction::Rational(sign, ratio) => {
                let (n, d): (F, F) = ratio.into();

                let numer = <T as TryToConvertFrom<F>>::try_to_convert_from(n)?;
                let denom = <T as TryToConvertFrom<F>>::try_to_convert_from(d)?;

                Some(GenericFraction::Rational(
                    sign,
                    super::Ratio::new_raw(numer, denom),
                ))
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/convert/tests_133.rs"));
