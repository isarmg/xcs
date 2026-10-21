use std::cmp;
use std::ops::MulAssign;

use crate::{decimal::GenericDecimal, generic::GenericInteger};

impl<O, T, P> MulAssign<O> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
    O: Into<Self>,
{
    fn mul_assign(&mut self, other: O) {
        let other: Self = other.into();

        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    MulAssign::mul_assign(sf, of);
                    *sp = cmp::max(*sp, op);
                }
            },
        };
    }
}

impl<'a, T, P> MulAssign<&'a Self> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
{
    fn mul_assign(&mut self, other: &'a Self) {
        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    MulAssign::mul_assign(sf, of);
                    *sp = cmp::max(*sp, *op);
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/decimal/ops/mul_assign/tests_43.rs"));
