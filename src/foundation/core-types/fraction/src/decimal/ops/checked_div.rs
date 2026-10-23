use CheckedDiv;

use crate::decimal::GenericDecimal;
use crate::generic::GenericInteger;
use std::cmp;

impl<T, P> CheckedDiv for GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
{
    fn checked_div(&self, other: &Self) -> Option<Self> {
        match *self {
            GenericDecimal(ref sf, sp) => match *other {
                GenericDecimal(ref of, op) => {
                    CheckedDiv::checked_div(sf, of).map(|val| GenericDecimal(val, cmp::max(sp, op)))
                }
            },
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/decimal/ops/checked_div/tests_23.rs"));
