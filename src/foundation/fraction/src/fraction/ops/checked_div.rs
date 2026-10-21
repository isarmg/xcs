use crate::fraction::{GenericFraction, Sign};
use crate::{CheckedDiv, CheckedMul, Integer, Ratio, Zero};

impl<T> CheckedDiv for GenericFraction<T>
where
    T: Clone + Integer + CheckedDiv + CheckedMul,
{
    fn checked_div(&self, other: &Self) -> Option<GenericFraction<T>> {
        match *self {
            GenericFraction::NaN => Some(self.clone()),
            GenericFraction::Infinity(sign) => match *other {
                GenericFraction::NaN => Some(other.clone()),
                GenericFraction::Infinity(_) => Some(GenericFraction::NaN),
                GenericFraction::Rational(osign, _) => {
                    Some(GenericFraction::Infinity(if sign == osign {
                        Sign::Plus
                    } else {
                        Sign::Minus
                    }))
                }
            },
            GenericFraction::Rational(sign, ref l) => match *other {
                GenericFraction::NaN => Some(other.clone()),
                GenericFraction::Infinity(_) => {
                    Some(GenericFraction::Rational(Sign::Plus, Ratio::zero()))
                }
                GenericFraction::Rational(osign, ref r) => {
                    if l.is_zero() && r.is_zero() {
                        Some(GenericFraction::NaN)
                    } else if r.is_zero() {
                        Some(GenericFraction::Infinity(sign))
                    } else if l.is_zero() {
                        Some(GenericFraction::Rational(Sign::Plus, l.clone()))
                    } else {
                        l.checked_div(r).map(|value| {
                            GenericFraction::Rational(
                                if sign == osign {
                                    Sign::Plus
                                } else {
                                    Sign::Minus
                                },
                                value,
                            )
                        })
                    }
                }
            },
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/ops/checked_div/tests_52.rs"));
