use crate::fraction::{GenericFraction, Sign};
use crate::{Integer, Ratio, Zero};
use std::mem;
use std::ops::{Mul, MulAssign};

impl<T, O> MulAssign<O> for GenericFraction<T>
where
    T: Clone + Integer,
    O: Into<GenericFraction<T>>,
{
    fn mul_assign(&mut self, other: O) {
        let other = other.into();

        *self = match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(ls) => match other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(rs) => {
                    GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                }
                GenericFraction::Rational(rs, r) => {
                    if r.is_zero() {
                        GenericFraction::NaN
                    } else {
                        GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                    }
                }
            },
            GenericFraction::Rational(ls, ref mut l) => match other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(rs) => {
                    if l.is_zero() {
                        GenericFraction::NaN
                    } else {
                        GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                    }
                }
                GenericFraction::Rational(rs, r) => {
                    let l_ = mem::replace(l, Ratio::new_raw(T::zero(), T::zero()));

                    let s = if l_.is_zero() || r.is_zero() || ls == rs {
                        Sign::Plus
                    } else {
                        Sign::Minus
                    };

                    GenericFraction::Rational(s, l_.mul(r))
                }
            },
        };
    }
}

impl<'a, T> MulAssign<&'a GenericFraction<T>> for GenericFraction<T>
where
    T: Clone + Integer,
{
    fn mul_assign(&mut self, other: &'a Self) {
        *self = match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(ls) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(rs) => {
                    GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                }
                GenericFraction::Rational(rs, ref r) => {
                    if r.is_zero() {
                        GenericFraction::NaN
                    } else {
                        GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                    }
                }
            },
            GenericFraction::Rational(ls, ref mut l) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(rs) => {
                    if l.is_zero() {
                        GenericFraction::NaN
                    } else {
                        GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                    }
                }
                GenericFraction::Rational(rs, ref r) => {
                    let l_ = mem::replace(l, Ratio::new_raw(T::zero(), T::zero()));

                    let s = if l_.is_zero() || r.is_zero() || ls == rs {
                        Sign::Plus
                    } else {
                        Sign::Minus
                    };

                    GenericFraction::Rational(s, l_.mul(r))
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/ops/mul_assign/tests_99.rs"));
