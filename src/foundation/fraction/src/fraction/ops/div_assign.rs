use crate::fraction::{GenericFraction, Sign};
use crate::{Integer, Ratio, Zero};
use std::mem;
use std::ops::{Div, DivAssign};

impl<T, O> DivAssign<O> for GenericFraction<T>
where
    T: Clone + Integer,
    O: Into<GenericFraction<T>>,
{
    fn div_assign(&mut self, other: O) {
        let other = other.into();

        *self = match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(ls) => match other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(_) => GenericFraction::NaN,
                GenericFraction::Rational(rs, _) => {
                    GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                }
            },
            GenericFraction::Rational(ls, ref mut l) => match other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(_) => {
                    GenericFraction::Rational(Sign::Plus, Ratio::zero())
                }
                GenericFraction::Rational(rs, r) => {
                    let l_ = mem::replace(l, Ratio::new_raw(T::zero(), T::zero()));

                    if l_.is_zero() && r.is_zero() {
                        GenericFraction::NaN
                    } else if r.is_zero() {
                        GenericFraction::Infinity(ls)
                    } else if l_.is_zero() {
                        GenericFraction::Rational(Sign::Plus, l_)
                    } else {
                        GenericFraction::Rational(
                            if ls == rs { Sign::Plus } else { Sign::Minus },
                            l_.div(r),
                        )
                    }
                }
            },
        };
    }
}

impl<'a, T> DivAssign<&'a GenericFraction<T>> for GenericFraction<T>
where
    T: Clone + Integer,
{
    fn div_assign(&mut self, other: &'a Self) {
        *self = match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(ls) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(_) => GenericFraction::NaN,
                GenericFraction::Rational(rs, _) => {
                    GenericFraction::Infinity(if ls == rs { Sign::Plus } else { Sign::Minus })
                }
            },
            GenericFraction::Rational(ls, ref mut l) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(_) => {
                    GenericFraction::Rational(Sign::Plus, Ratio::zero())
                }
                GenericFraction::Rational(rs, ref r) => {
                    let l_ = mem::replace(l, Ratio::new_raw(T::zero(), T::zero()));

                    if l_.is_zero() && r.is_zero() {
                        GenericFraction::NaN
                    } else if r.is_zero() {
                        GenericFraction::Infinity(ls)
                    } else if l_.is_zero() {
                        GenericFraction::Rational(Sign::Plus, l_)
                    } else {
                        GenericFraction::Rational(
                            if ls == rs { Sign::Plus } else { Sign::Minus },
                            l_.div(r),
                        )
                    }
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/ops/div_assign/tests_89.rs"));
