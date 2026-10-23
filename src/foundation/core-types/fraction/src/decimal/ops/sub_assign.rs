use std::cmp;
use std::ops::SubAssign;

use crate::{decimal::GenericDecimal, generic::GenericInteger};

impl<O, T, P> SubAssign<O> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
    O: Into<Self>,
{
    fn sub_assign(&mut self, other: O) {
        let other: Self = other.into();

        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    SubAssign::sub_assign(sf, of);
                    *sp = cmp::max(*sp, op);
                }
            },
        };
    }
}

impl<'a, T, P> SubAssign<&'a Self> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger, // + $trait + $trait<&'a T>,
    P: Copy + GenericInteger + Into<usize>,
{
    fn sub_assign(&mut self, other: &'a Self) {
        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    SubAssign::sub_assign(sf, of);
                    *sp = cmp::max(*sp, *op);
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/decimal/ops/sub_assign/tests_43.rs"));
