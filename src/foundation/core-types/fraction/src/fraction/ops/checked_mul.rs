use crate::fraction::{GenericFraction, Sign};
use crate::{CheckedMul, Integer, Zero};

impl<T> CheckedMul for GenericFraction<T>
where
    T: Clone + Integer + CheckedMul,
{
    fn checked_mul(&self, other: &Self) -> Option<GenericFraction<T>> {
        match *self {
            GenericFraction::NaN => Some(self.clone()),
            GenericFraction::Infinity(sign) => match *other {
                GenericFraction::NaN => Some(other.clone()),
                GenericFraction::Infinity(osign) => {
                    Some(GenericFraction::Infinity(if sign == osign {
                        Sign::Plus
                    } else {
                        Sign::Minus
                    }))
                }
                GenericFraction::Rational(osign, ref l) => {
                    if l.is_zero() {
                        Some(GenericFraction::NaN)
                    } else {
                        Some(GenericFraction::Infinity(if sign == osign {
                            Sign::Plus
                        } else {
                            Sign::Minus
                        }))
                    }
                }
            },
            GenericFraction::Rational(sign, ref l) => match *other {
                GenericFraction::NaN => Some(other.clone()),
                GenericFraction::Infinity(osign) => {
                    if l.is_zero() {
                        Some(GenericFraction::NaN)
                    } else {
                        Some(GenericFraction::Infinity(if sign == osign {
                            Sign::Plus
                        } else {
                            Sign::Minus
                        }))
                    }
                }
                GenericFraction::Rational(osign, ref r) => l.checked_mul(r).map(|value| {
                    GenericFraction::Rational(
                        if l.is_zero() || r.is_zero() || sign == osign {
                            Sign::Plus
                        } else {
                            Sign::Minus
                        },
                        value,
                    )
                }),
            },
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/fraction/ops/checked_mul/tests_60.rs"));
