use std::cmp;
use std::ops::{AddAssign, MulAssign};

use crate::{decimal::GenericDecimal, generic::GenericInteger};

impl<O, T, P> AddAssign<O> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger + AddAssign + AddAssign + MulAssign,
    P: Copy + GenericInteger + Into<usize>,
    O: Into<GenericDecimal<T, P>>,
{
    fn add_assign(&mut self, other: O) {
        let other: GenericDecimal<T, P> = other.into();

        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    AddAssign::add_assign(sf, of);
                    *sp = cmp::max(*sp, op);
                }
            },
        };
    }
}

impl<'a, T, P> AddAssign<&'a Self> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger + AddAssign,
    P: Copy + GenericInteger + Into<usize>,
{
    fn add_assign(&mut self, other: &'a Self) {
        match *self {
            GenericDecimal(ref mut sf, ref mut sp) => match other {
                GenericDecimal(of, op) => {
                    AddAssign::add_assign(sf, of);
                    *sp = cmp::max(*sp, *op);
                }
            },
        };
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/decimal/ops/add_assign/tests_43.rs"));
