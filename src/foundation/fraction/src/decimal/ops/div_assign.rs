use crate::{decimal::GenericDecimal, generic::GenericInteger};

use std::cmp;
use std::ops::DivAssign;

impl<O, T, P> DivAssign<O> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
    O: Into<Self>,
{
    fn div_assign(&mut self, other: O) {
        let other: Self = other.into();

        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    DivAssign::div_assign(sf, of);
                    *sp = cmp::max(*sp, op);
                }
            },
        };
    }
}

impl<'a, T, P> DivAssign<&'a Self> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger + DivAssign,
    P: Copy + GenericInteger + Into<usize>,
{
    fn div_assign(&mut self, other: &'a Self) {
        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    DivAssign::div_assign(sf, of);
                    *sp = cmp::max(*sp, *op);
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/decimal/ops/div_assign/tests_43.rs"));
