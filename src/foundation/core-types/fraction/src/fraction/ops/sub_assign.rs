use crate::fraction::{GenericFraction, Sign};
use crate::{Integer, Ratio};
use std::mem;
use std::ops::{Add, Sub, SubAssign};

impl<T, O> SubAssign<O> for GenericFraction<T>
where
    T: Clone + Integer,
    O: Into<GenericFraction<T>>,
{
    fn sub_assign(&mut self, other: O) {
        let other = other.into();

        *self = match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(ls) => match other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(rs) => {
                    if ls == rs {
                        GenericFraction::NaN
                    } else {
                        GenericFraction::Infinity(ls)
                    }
                }
                GenericFraction::Rational(_, _) => GenericFraction::Infinity(ls),
            },
            GenericFraction::Rational(ls, ref mut l) => match other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(s) => GenericFraction::Infinity(-s),
                GenericFraction::Rational(rs, r) => {
                    let l_ = mem::replace(l, Ratio::new_raw(T::zero(), T::zero()));

                    if ls == Sign::Plus && rs == Sign::Plus {
                        if r > l_ {
                            GenericFraction::Rational(Sign::Minus, r.sub(l_))
                        } else {
                            GenericFraction::Rational(Sign::Plus, l_.sub(r))
                        }
                    } else if ls == Sign::Plus {
                        GenericFraction::Rational(Sign::Plus, l_.add(r))
                    } else if rs == Sign::Plus {
                        GenericFraction::Rational(Sign::Minus, l_.add(r))
                    } else if l_ > r {
                        GenericFraction::Rational(Sign::Minus, l_.sub(r))
                    } else {
                        GenericFraction::Rational(Sign::Plus, r.sub(l_))
                    }
                }
            },
        };
    }
}

impl<'a, T> SubAssign<&'a GenericFraction<T>> for GenericFraction<T>
where
    T: Clone + Integer,
{
    fn sub_assign(&mut self, other: &'a Self) {
        *self = match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(ls) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(rs) => {
                    if ls == rs {
                        GenericFraction::NaN
                    } else {
                        GenericFraction::Infinity(ls)
                    }
                }
                GenericFraction::Rational(_, _) => GenericFraction::Infinity(ls),
            },
            GenericFraction::Rational(ls, ref mut l) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(s) => GenericFraction::Infinity(-s),
                GenericFraction::Rational(rs, ref r) => {
                    let l_ = mem::replace(l, Ratio::new_raw(T::zero(), T::zero()));

                    if ls == Sign::Plus && rs == Sign::Plus {
                        if l_ < *r {
                            GenericFraction::Rational(Sign::Minus, r.sub(l_))
                        } else {
                            GenericFraction::Rational(Sign::Plus, l_.sub(r))
                        }
                    } else if ls == Sign::Plus {
                        GenericFraction::Rational(Sign::Plus, l_.add(r))
                    } else if rs == Sign::Plus {
                        GenericFraction::Rational(Sign::Minus, l_.add(r))
                    } else if l_ > *r {
                        GenericFraction::Rational(Sign::Minus, l_.sub(r))
                    } else {
                        GenericFraction::Rational(Sign::Plus, r.sub(l_))
                    }
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/fraction/ops/sub_assign/tests_99.rs"));
