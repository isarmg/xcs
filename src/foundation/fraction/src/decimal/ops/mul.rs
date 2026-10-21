use crate::generic::GenericInteger;
use crate::GenericDecimal;
use std::cmp;
use std::ops::Mul;

impl<O, T, P> Mul<O> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
    O: Into<GenericDecimal<T, P>>,
{
    type Output = Self;

    fn mul(self, other: O) -> Self::Output {
        let other: GenericDecimal<T, P> = other.into();

        match self {
            GenericDecimal(sf, sp) => match other {
                GenericDecimal(of, op) => GenericDecimal(Mul::mul(sf, of), cmp::max(sp, op)),
            },
        }
    }
}

impl<'a, T, P> Mul for &'a GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
    &'a T: Mul<Output = T>,
{
    type Output = GenericDecimal<T, P>;

    fn mul(self, other: Self) -> Self::Output {
        match self {
            GenericDecimal(sf, sp) => match other {
                GenericDecimal(of, op) => GenericDecimal(Mul::mul(sf, of), cmp::max(*sp, *op)),
            },
        }
    }
}

impl<'a, O, T, P> Mul<O> for &'a GenericDecimal<T, P>
where
    T: Clone + GenericInteger,
    P: Copy + GenericInteger + Into<usize>,
    &'a T: Mul<Output = T>,
    O: Into<GenericDecimal<T, P>>,
{
    type Output = GenericDecimal<T, P>;

    fn mul(self, other: O) -> Self::Output {
        let other: GenericDecimal<T, P> = other.into();

        match self {
            GenericDecimal(sf, sp) => match other {
                GenericDecimal(of, op) => GenericDecimal(Mul::mul(sf, of), cmp::max(*sp, op)),
            },
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/decimal/ops/mul/tests_62.rs"));
