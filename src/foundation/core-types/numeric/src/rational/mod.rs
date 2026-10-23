// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Rational numbers
//!
//! ## Compatibility
//!
//! The `num-rational` crate is tested for rustc 1.60 and greater.

// Ratio ops often use other "suspicious" ops
#![allow(clippy::suspicious_arithmetic_impl)]
#![allow(clippy::suspicious_op_assign_impl)]


use core::cmp;
use core::fmt;
use core::fmt::{Binary, Display, Formatter, LowerExp, LowerHex, Octal, UpperExp, UpperHex};
use core::hash::{Hash, Hasher};
use core::ops::{Add, Div, Mul, Neg, Rem, ShlAssign, Sub};
use core::str::FromStr;
#[cfg(feature = "std")]
use std::error::Error;

#[cfg(feature = "bigint")]
use crate::bigint::{BigInt, BigUint, Sign, ToBigInt};

use crate::integer::Integer;
use crate::traits::float::FloatCore;
use crate::traits::{
    Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, ConstOne, ConstZero, FromPrimitive,
    Inv, Num, NumCast, One, Pow, Signed, ToPrimitive, Unsigned, Zero,
};

mod pow;

/// Represents the ratio between two numbers.
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub struct Ratio<T> {
    /// Numerator.
    numer: T,
    /// Denominator.
    denom: T,
}

/// Alias for a `Ratio` of machine-sized integers.
#[deprecated(
    since = "0.4.0",
    note = "it's better to use a specific size, like `Rational32` or `Rational64`"
)]
pub type Rational = Ratio<isize>;
/// Alias for a `Ratio` of 32-bit-sized integers.
pub type Rational32 = Ratio<i32>;
/// Alias for a `Ratio` of 64-bit-sized integers.
pub type Rational64 = Ratio<i64>;

#[cfg(feature = "bigint")]
/// Alias for arbitrary precision rationals.
pub type BigRational = Ratio<BigInt>;

/// These method are `const`.
impl<T> Ratio<T> {
    /// Creates a `Ratio` without checking for `denom == 0` or reducing.
    ///
    /// **There are several methods that will panic if used on a `Ratio` with
    /// `denom == 0`.**
    #[inline]
    pub const fn new_raw(numer: T, denom: T) -> Ratio<T> {
        Ratio { numer, denom }
    }

    /// Deconstructs a `Ratio` into its numerator and denominator.
    #[inline]
    pub fn into_raw(self) -> (T, T) {
        (self.numer, self.denom)
    }

    /// Gets an immutable reference to the numerator.
    #[inline]
    pub const fn numer(&self) -> &T {
        &self.numer
    }

    /// Gets an immutable reference to the denominator.
    #[inline]
    pub const fn denom(&self) -> &T {
        &self.denom
    }
}

impl<T: Clone + Integer> Ratio<T> {
    /// Creates a new `Ratio`.
    ///
    /// **Panics if `denom` is zero.**
    #[inline]
    pub fn new(numer: T, denom: T) -> Ratio<T> {
        let mut ret = Ratio::new_raw(numer, denom);
        ret.reduce();
        ret
    }

    /// Creates a `Ratio` representing the integer `t`.
    #[inline]
    pub fn from_integer(t: T) -> Ratio<T> {
        Ratio::new_raw(t, One::one())
    }

    /// Converts to an integer, rounding towards zero.
    #[inline]
    pub fn to_integer(&self) -> T {
        self.trunc().numer
    }

    /// Returns true if the rational number is an integer (denominator is 1).
    #[inline]
    pub fn is_integer(&self) -> bool {
        self.denom.is_one()
    }

    /// Puts self into lowest terms, with `denom` > 0.
    ///
    /// **Panics if `denom` is zero.**
    fn reduce(&mut self) {
        if self.denom.is_zero() {
            panic!("denominator == 0");
        }
        if self.numer.is_zero() {
            self.denom.set_one();
            return;
        }
        if self.numer == self.denom {
            self.set_one();
            return;
        }
        let g: T = self.numer.gcd(&self.denom);

        // FIXME(#5992): assignment operator overloads
        // T: Clone + Integer != T: Clone + NumAssign

        #[inline]
        fn replace_with<T: Zero>(x: &mut T, f: impl FnOnce(T) -> T) {
            let y = core::mem::replace(x, T::zero());
            *x = f(y);
        }

        // self.numer /= g;
        replace_with(&mut self.numer, |x| x / g.clone());

        // self.denom /= g;
        replace_with(&mut self.denom, |x| x / g);

        // keep denom positive!
        if self.denom < T::zero() {
            replace_with(&mut self.numer, |x| T::zero() - x);
            replace_with(&mut self.denom, |x| T::zero() - x);
        }
    }

    /// Returns a reduced copy of self.
    ///
    /// In general, it is not necessary to use this method, as the only
    /// method of procuring a non-reduced fraction is through `new_raw`.
    ///
    /// **Panics if `denom` is zero.**
    pub fn reduced(&self) -> Ratio<T> {
        let mut ret = self.clone();
        ret.reduce();
        ret
    }

    /// Returns the reciprocal.
    ///
    /// **Panics if the `Ratio` is zero.**
    #[inline]
    pub fn recip(&self) -> Ratio<T> {
        self.clone().into_recip()
    }

    #[inline]
    fn into_recip(self) -> Ratio<T> {
        match self.numer.cmp(&T::zero()) {
            cmp::Ordering::Equal => panic!("division by zero"),
            cmp::Ordering::Greater => Ratio::new_raw(self.denom, self.numer),
            cmp::Ordering::Less => Ratio::new_raw(T::zero() - self.denom, T::zero() - self.numer),
        }
    }

    /// Rounds towards minus infinity.
    #[inline]
    pub fn floor(&self) -> Ratio<T> {
        if *self < Zero::zero() {
            let one: T = One::one();
            Ratio::from_integer(
                (self.numer.clone() - self.denom.clone() + one) / self.denom.clone(),
            )
        } else {
            Ratio::from_integer(self.numer.clone() / self.denom.clone())
        }
    }

    /// Rounds towards plus infinity.
    #[inline]
    pub fn ceil(&self) -> Ratio<T> {
        if *self < Zero::zero() {
            Ratio::from_integer(self.numer.clone() / self.denom.clone())
        } else {
            let one: T = One::one();
            Ratio::from_integer(
                (self.numer.clone() + self.denom.clone() - one) / self.denom.clone(),
            )
        }
    }

    /// Rounds to the nearest integer. Rounds half-way cases away from zero.
    #[inline]
    pub fn round(&self) -> Ratio<T> {
        let zero: Ratio<T> = Zero::zero();
        let one: T = One::one();
        let two: T = one.clone() + one.clone();

        // Find unsigned fractional part of rational number
        let mut fractional = self.fract();
        if fractional < zero {
            fractional = zero - fractional
        };

        // The algorithm compares the unsigned fractional part with 1/2, that
        // is, a/b >= 1/2, or a >= b/2. For odd denominators, we use
        // a >= (b/2)+1. This avoids overflow issues.
        let half_or_larger = if fractional.denom.is_even() {
            fractional.numer >= fractional.denom / two
        } else {
            fractional.numer >= (fractional.denom / two) + one
        };

        if half_or_larger {
            let one: Ratio<T> = One::one();
            if *self >= Zero::zero() {
                self.trunc() + one
            } else {
                self.trunc() - one
            }
        } else {
            self.trunc()
        }
    }

    /// Rounds towards zero.
    #[inline]
    pub fn trunc(&self) -> Ratio<T> {
        Ratio::from_integer(self.numer.clone() / self.denom.clone())
    }

    /// Returns the fractional part of a number, with division rounded towards zero.
    ///
    /// Satisfies `self == self.trunc() + self.fract()`.
    #[inline]
    pub fn fract(&self) -> Ratio<T> {
        Ratio::new_raw(self.numer.clone() % self.denom.clone(), self.denom.clone())
    }

    /// Raises the `Ratio` to the power of an exponent.
    #[inline]
    pub fn pow(&self, expon: i32) -> Ratio<T>
    where
        for<'a> &'a T: Pow<u32, Output = T>,
    {
        Pow::pow(self, expon)
    }
}

#[cfg(feature = "bigint")]
impl Ratio<BigInt> {
    /// Converts a float into a rational number.
    pub fn from_float<T: FloatCore>(f: T) -> Option<BigRational> {
        if !f.is_finite() {
            return None;
        }
        let (mantissa, exponent, sign) = f.integer_decode();
        let bigint_sign = if sign == 1 { Sign::Plus } else { Sign::Minus };
        if exponent < 0 {
            let one: BigInt = One::one();
            let denom: BigInt = one << ((-exponent) as usize);
            let numer: BigUint = FromPrimitive::from_u64(mantissa).unwrap();
            Some(Ratio::new(BigInt::from_biguint(bigint_sign, numer), denom))
        } else {
            let mut numer: BigUint = FromPrimitive::from_u64(mantissa).unwrap();
            numer <<= exponent as usize;
            Some(Ratio::from_integer(BigInt::from_biguint(
                bigint_sign,
                numer,
            )))
        }
    }
}

impl<T: Clone + Integer> Default for Ratio<T> {
    /// Returns zero
    fn default() -> Self {
        Ratio::zero()
    }
}

// From integer
impl<T> From<T> for Ratio<T>
where
    T: Clone + Integer,
{
    fn from(x: T) -> Ratio<T> {
        Ratio::from_integer(x)
    }
}

// From pair (through the `new` constructor)
impl<T> From<(T, T)> for Ratio<T>
where
    T: Clone + Integer,
{
    fn from(pair: (T, T)) -> Ratio<T> {
        Ratio::new(pair.0, pair.1)
    }
}

// Comparisons

// Mathematically, comparing a/b and c/d is the same as comparing a*d and b*c, but it's very easy
// for those multiplications to overflow fixed-size integers, so we need to take care.

impl<T: Clone + Integer> Ord for Ratio<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        // With equal denominators, the numerators can be directly compared
        if self.denom == other.denom {
            let ord = self.numer.cmp(&other.numer);
            return if self.denom < T::zero() {
                ord.reverse()
            } else {
                ord
            };
        }

        // With equal numerators, the denominators can be inversely compared
        if self.numer == other.numer {
            if self.numer.is_zero() {
                return cmp::Ordering::Equal;
            }
            let ord = self.denom.cmp(&other.denom);
            return if self.numer < T::zero() {
                ord
            } else {
                ord.reverse()
            };
        }

        // Unfortunately, we don't have CheckedMul to try.  That could sometimes avoid all the
        // division below, or even always avoid it for BigInt and BigUint.
        // FIXME- future breaking change to add Checked* to Integer?

        // Compare as floored integers and remainders
        let (self_int, self_rem) = self.numer.div_mod_floor(&self.denom);
        let (other_int, other_rem) = other.numer.div_mod_floor(&other.denom);
        match self_int.cmp(&other_int) {
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Equal => {
                match (self_rem.is_zero(), other_rem.is_zero()) {
                    (true, true) => cmp::Ordering::Equal,
                    (true, false) => cmp::Ordering::Less,
                    (false, true) => cmp::Ordering::Greater,
                    (false, false) => {
                        // Compare the reciprocals of the remaining fractions in reverse
                        let self_recip = Ratio::new_raw(self.denom.clone(), self_rem);
                        let other_recip = Ratio::new_raw(other.denom.clone(), other_rem);
                        self_recip.cmp(&other_recip).reverse()
                    }
                }
            }
        }
    }
}

impl<T: Clone + Integer> PartialOrd for Ratio<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Clone + Integer> PartialEq for Ratio<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<T: Clone + Integer> Eq for Ratio<T> {}

// NB: We can't just `#[derive(Hash)]`, because it needs to agree
// with `Eq` even for non-reduced ratios.
impl<T: Clone + Integer + Hash> Hash for Ratio<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        recurse(&self.numer, &self.denom, state);

        fn recurse<T: Integer + Hash, H: Hasher>(numer: &T, denom: &T, state: &mut H) {
            if !denom.is_zero() {
                let (int, rem) = numer.div_mod_floor(denom);
                int.hash(state);
                recurse(denom, &rem, state);
            } else {
                denom.hash(state);
            }
        }
    }
}

mod iter_sum_product {
    use crate::rational::Ratio;
    use core::iter::{Product, Sum};
    use crate::integer::Integer;
    use crate::traits::{One, Zero};

    impl<T: Integer + Clone> Sum for Ratio<T> {
        fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = Ratio<T>>,
        {
            iter.fold(Self::zero(), |sum, num| sum + num)
        }
    }

    impl<'a, T: Integer + Clone> Sum<&'a Ratio<T>> for Ratio<T> {
        fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = &'a Ratio<T>>,
        {
            iter.fold(Self::zero(), |sum, num| sum + num)
        }
    }

    impl<T: Integer + Clone> Product for Ratio<T> {
        fn product<I>(iter: I) -> Self
        where
            I: Iterator<Item = Ratio<T>>,
        {
            iter.fold(Self::one(), |prod, num| prod * num)
        }
    }

    impl<'a, T: Integer + Clone> Product<&'a Ratio<T>> for Ratio<T> {
        fn product<I>(iter: I) -> Self
        where
            I: Iterator<Item = &'a Ratio<T>>,
        {
            iter.fold(Self::one(), |prod, num| prod * num)
        }
    }
}

mod opassign {
    use core::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

    use crate::rational::Ratio;
    use crate::integer::Integer;
    use crate::traits::NumAssign;

    impl<T: Clone + Integer + NumAssign> AddAssign for Ratio<T> {
        fn add_assign(&mut self, other: Ratio<T>) {
            if self.denom == other.denom {
                self.numer += other.numer
            } else {
                let lcm = self.denom.lcm(&other.denom);
                let lhs_numer = self.numer.clone() * (lcm.clone() / self.denom.clone());
                let rhs_numer = other.numer * (lcm.clone() / other.denom);
                self.numer = lhs_numer + rhs_numer;
                self.denom = lcm;
            }
            self.reduce();
        }
    }

    // (a/b) / (c/d) = (a/gcd_ac)*(d/gcd_bd) / ((c/gcd_ac)*(b/gcd_bd))
    impl<T: Clone + Integer + NumAssign> DivAssign for Ratio<T> {
        fn div_assign(&mut self, other: Ratio<T>) {
            let gcd_ac = self.numer.gcd(&other.numer);
            let gcd_bd = self.denom.gcd(&other.denom);
            self.numer /= gcd_ac.clone();
            self.numer *= other.denom / gcd_bd.clone();
            self.denom /= gcd_bd;
            self.denom *= other.numer / gcd_ac;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    // a/b * c/d = (a/gcd_ad)*(c/gcd_bc) / ((d/gcd_ad)*(b/gcd_bc))
    impl<T: Clone + Integer + NumAssign> MulAssign for Ratio<T> {
        fn mul_assign(&mut self, other: Ratio<T>) {
            let gcd_ad = self.numer.gcd(&other.denom);
            let gcd_bc = self.denom.gcd(&other.numer);
            self.numer /= gcd_ad.clone();
            self.numer *= other.numer / gcd_bc.clone();
            self.denom /= gcd_bc;
            self.denom *= other.denom / gcd_ad;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    impl<T: Clone + Integer + NumAssign> RemAssign for Ratio<T> {
        fn rem_assign(&mut self, other: Ratio<T>) {
            if self.denom == other.denom {
                self.numer %= other.numer
            } else {
                let lcm = self.denom.lcm(&other.denom);
                let lhs_numer = self.numer.clone() * (lcm.clone() / self.denom.clone());
                let rhs_numer = other.numer * (lcm.clone() / other.denom);
                self.numer = lhs_numer % rhs_numer;
                self.denom = lcm;
            }
            self.reduce();
        }
    }

    impl<T: Clone + Integer + NumAssign> SubAssign for Ratio<T> {
        fn sub_assign(&mut self, other: Ratio<T>) {
            if self.denom == other.denom {
                self.numer -= other.numer
            } else {
                let lcm = self.denom.lcm(&other.denom);
                let lhs_numer = self.numer.clone() * (lcm.clone() / self.denom.clone());
                let rhs_numer = other.numer * (lcm.clone() / other.denom);
                self.numer = lhs_numer - rhs_numer;
                self.denom = lcm;
            }
            self.reduce();
        }
    }

    // a/b + c/1 = (a*1 + b*c) / (b*1) = (a + b*c) / b
    impl<T: Clone + Integer + NumAssign> AddAssign<T> for Ratio<T> {
        fn add_assign(&mut self, other: T) {
            self.numer += self.denom.clone() * other;
            self.reduce();
        }
    }

    impl<T: Clone + Integer + NumAssign> DivAssign<T> for Ratio<T> {
        fn div_assign(&mut self, other: T) {
            let gcd = self.numer.gcd(&other);
            self.numer /= gcd.clone();
            self.denom *= other / gcd;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    impl<T: Clone + Integer + NumAssign> MulAssign<T> for Ratio<T> {
        fn mul_assign(&mut self, other: T) {
            let gcd = self.denom.gcd(&other);
            self.denom /= gcd.clone();
            self.numer *= other / gcd;
            self.reduce(); // TODO: remove this line. see #8.
        }
    }

    // a/b % c/1 = (a*1 % b*c) / (b*1) = (a % b*c) / b
    impl<T: Clone + Integer + NumAssign> RemAssign<T> for Ratio<T> {
        fn rem_assign(&mut self, other: T) {
            self.numer %= self.denom.clone() * other;
            self.reduce();
        }
    }

    // a/b - c/1 = (a*1 - b*c) / (b*1) = (a - b*c) / b
    impl<T: Clone + Integer + NumAssign> SubAssign<T> for Ratio<T> {
        fn sub_assign(&mut self, other: T) {
            self.numer -= self.denom.clone() * other;
            self.reduce();
        }
    }

    macro_rules! forward_op_assign {
        (impl $imp:ident, $method:ident) => {
            impl<'a, T: Clone + Integer + NumAssign> $imp<&'a Ratio<T>> for Ratio<T> {
                #[inline]
                fn $method(&mut self, other: &Ratio<T>) {
                    self.$method(other.clone())
                }
            }
            impl<'a, T: Clone + Integer + NumAssign> $imp<&'a T> for Ratio<T> {
                #[inline]
                fn $method(&mut self, other: &T) {
                    self.$method(other.clone())
                }
            }
        };
    }

    forward_op_assign!(impl AddAssign, add_assign);
    forward_op_assign!(impl DivAssign, div_assign);
    forward_op_assign!(impl MulAssign, mul_assign);
    forward_op_assign!(impl RemAssign, rem_assign);
    forward_op_assign!(impl SubAssign, sub_assign);
}

macro_rules! forward_ref_ref_binop {
    (impl $imp:ident, $method:ident) => {
        impl<'a, 'b, T: Clone + Integer> $imp<&'b Ratio<T>> for &'a Ratio<T> {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &'b Ratio<T>) -> Ratio<T> {
                self.clone().$method(other.clone())
            }
        }
        impl<'a, 'b, T: Clone + Integer> $imp<&'b T> for &'a Ratio<T> {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &'b T) -> Ratio<T> {
                self.clone().$method(other.clone())
            }
        }
    };
}

macro_rules! forward_ref_val_binop {
    (impl $imp:ident, $method:ident) => {
        impl<'a, T> $imp<Ratio<T>> for &'a Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: Ratio<T>) -> Ratio<T> {
                self.clone().$method(other)
            }
        }
        impl<'a, T> $imp<T> for &'a Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: T) -> Ratio<T> {
                self.clone().$method(other)
            }
        }
    };
}

macro_rules! forward_val_ref_binop {
    (impl $imp:ident, $method:ident) => {
        impl<'a, T> $imp<&'a Ratio<T>> for Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &Ratio<T>) -> Ratio<T> {
                self.$method(other.clone())
            }
        }
        impl<'a, T> $imp<&'a T> for Ratio<T>
        where
            T: Clone + Integer,
        {
            type Output = Ratio<T>;

            #[inline]
            fn $method(self, other: &T) -> Ratio<T> {
                self.$method(other.clone())
            }
        }
    };
}

macro_rules! forward_all_binop {
    (impl $imp:ident, $method:ident) => {
        forward_ref_ref_binop!(impl $imp, $method);
        forward_ref_val_binop!(impl $imp, $method);
        forward_val_ref_binop!(impl $imp, $method);
    };
}

// Arithmetic
forward_all_binop!(impl Mul, mul);
// a/b * c/d = (a/gcd_ad)*(c/gcd_bc) / ((d/gcd_ad)*(b/gcd_bc))
impl<T> Mul<Ratio<T>> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;
    #[inline]
    fn mul(self, rhs: Ratio<T>) -> Ratio<T> {
        let gcd_ad = self.numer.gcd(&rhs.denom);
        let gcd_bc = self.denom.gcd(&rhs.numer);
        Ratio::new(
            self.numer / gcd_ad.clone() * (rhs.numer / gcd_bc.clone()),
            self.denom / gcd_bc * (rhs.denom / gcd_ad),
        )
    }
}
// a/b * c/1 = (a*c) / (b*1) = (a*c) / b
impl<T> Mul<T> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;
    #[inline]
    fn mul(self, rhs: T) -> Ratio<T> {
        let gcd = self.denom.gcd(&rhs);
        Ratio::new(self.numer * (rhs / gcd.clone()), self.denom / gcd)
    }
}

forward_all_binop!(impl Div, div);
// (a/b) / (c/d) = (a/gcd_ac)*(d/gcd_bd) / ((c/gcd_ac)*(b/gcd_bd))
impl<T> Div<Ratio<T>> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn div(self, rhs: Ratio<T>) -> Ratio<T> {
        let gcd_ac = self.numer.gcd(&rhs.numer);
        let gcd_bd = self.denom.gcd(&rhs.denom);
        Ratio::new(
            self.numer / gcd_ac.clone() * (rhs.denom / gcd_bd.clone()),
            self.denom / gcd_bd * (rhs.numer / gcd_ac),
        )
    }
}
// (a/b) / (c/1) = (a*1) / (b*c) = a / (b*c)
impl<T> Div<T> for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn div(self, rhs: T) -> Ratio<T> {
        let gcd = self.numer.gcd(&rhs);
        Ratio::new(self.numer / gcd.clone(), self.denom * (rhs / gcd))
    }
}

macro_rules! arith_impl {
    (impl $imp:ident, $method:ident) => {
        forward_all_binop!(impl $imp, $method);
        // Abstracts a/b `op` c/d = (a*lcm/b `op` c*lcm/d)/lcm where lcm = lcm(b,d)
        impl<T: Clone + Integer> $imp<Ratio<T>> for Ratio<T> {
            type Output = Ratio<T>;
            #[inline]
            fn $method(self, rhs: Ratio<T>) -> Ratio<T> {
                if self.denom == rhs.denom {
                    return Ratio::new(self.numer.$method(rhs.numer), rhs.denom);
                }
                let lcm = self.denom.lcm(&rhs.denom);
                let lhs_numer = self.numer * (lcm.clone() / self.denom);
                let rhs_numer = rhs.numer * (lcm.clone() / rhs.denom);
                Ratio::new(lhs_numer.$method(rhs_numer), lcm)
            }
        }
        // Abstracts the a/b `op` c/1 = (a*1 `op` b*c) / (b*1) = (a `op` b*c) / b pattern
        impl<T: Clone + Integer> $imp<T> for Ratio<T> {
            type Output = Ratio<T>;
            #[inline]
            fn $method(self, rhs: T) -> Ratio<T> {
                Ratio::new(self.numer.$method(self.denom.clone() * rhs), self.denom)
            }
        }
    };
}

arith_impl!(impl Add, add);
arith_impl!(impl Sub, sub);
arith_impl!(impl Rem, rem);

// a/b * c/d = (a*c)/(b*d)
impl<T> CheckedMul for Ratio<T>
where
    T: Clone + Integer + CheckedMul,
{
    #[inline]
    fn checked_mul(&self, rhs: &Ratio<T>) -> Option<Ratio<T>> {
        let gcd_ad = self.numer.gcd(&rhs.denom);
        let gcd_bc = self.denom.gcd(&rhs.numer);
        Some(Ratio::new(
            (self.numer.clone() / gcd_ad.clone())
                .checked_mul(&(rhs.numer.clone() / gcd_bc.clone()))?,
            (self.denom.clone() / gcd_bc).checked_mul(&(rhs.denom.clone() / gcd_ad))?,
        ))
    }
}

// (a/b) / (c/d) = (a*d)/(b*c)
impl<T> CheckedDiv for Ratio<T>
where
    T: Clone + Integer + CheckedMul,
{
    #[inline]
    fn checked_div(&self, rhs: &Ratio<T>) -> Option<Ratio<T>> {
        if rhs.is_zero() {
            return None;
        }
        let (numer, denom) = if self.denom == rhs.denom {
            (self.numer.clone(), rhs.numer.clone())
        } else if self.numer == rhs.numer {
            (rhs.denom.clone(), self.denom.clone())
        } else {
            let gcd_ac = self.numer.gcd(&rhs.numer);
            let gcd_bd = self.denom.gcd(&rhs.denom);
            (
                (self.numer.clone() / gcd_ac.clone())
                    .checked_mul(&(rhs.denom.clone() / gcd_bd.clone()))?,
                (self.denom.clone() / gcd_bd).checked_mul(&(rhs.numer.clone() / gcd_ac))?,
            )
        };
        // Manual `reduce()`, avoiding sharp edges
        if denom.is_zero() {
            None
        } else if numer.is_zero() {
            Some(Self::zero())
        } else if numer == denom {
            Some(Self::one())
        } else {
            let g = numer.gcd(&denom);
            let numer = numer / g.clone();
            let denom = denom / g;
            let raw = if denom < T::zero() {
                // We need to keep denom positive, but 2's-complement MIN may
                // overflow negation -- instead we can check multiplying -1.
                let n1 = T::zero() - T::one();
                Ratio::new_raw(numer.checked_mul(&n1)?, denom.checked_mul(&n1)?)
            } else {
                Ratio::new_raw(numer, denom)
            };
            Some(raw)
        }
    }
}

// As arith_impl! but for Checked{Add,Sub} traits
macro_rules! checked_arith_impl {
    (impl $imp:ident, $method:ident) => {
        impl<T: Clone + Integer + CheckedMul + $imp> $imp for Ratio<T> {
            #[inline]
            fn $method(&self, rhs: &Ratio<T>) -> Option<Ratio<T>> {
                let gcd = self.denom.clone().gcd(&rhs.denom);
                let lcm = (self.denom.clone() / gcd.clone()).checked_mul(&rhs.denom)?;
                let lhs_numer = (lcm.clone() / self.denom.clone()).checked_mul(&self.numer)?;
                let rhs_numer = (lcm.clone() / rhs.denom.clone()).checked_mul(&rhs.numer)?;
                Some(Ratio::new(lhs_numer.$method(&rhs_numer)?, lcm))
            }
        }
    };
}

// a/b + c/d = (lcm/b*a + lcm/d*c)/lcm, where lcm = lcm(b,d)
checked_arith_impl!(impl CheckedAdd, checked_add);

// a/b - c/d = (lcm/b*a - lcm/d*c)/lcm, where lcm = lcm(b,d)
checked_arith_impl!(impl CheckedSub, checked_sub);

impl<T> Neg for Ratio<T>
where
    T: Clone + Integer + Neg<Output = T>,
{
    type Output = Ratio<T>;

    #[inline]
    fn neg(self) -> Ratio<T> {
        Ratio::new_raw(-self.numer, self.denom)
    }
}

impl<'a, T> Neg for &'a Ratio<T>
where
    T: Clone + Integer + Neg<Output = T>,
{
    type Output = Ratio<T>;

    #[inline]
    fn neg(self) -> Ratio<T> {
        -self.clone()
    }
}

impl<T> Inv for Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn inv(self) -> Ratio<T> {
        self.recip()
    }
}

impl<'a, T> Inv for &'a Ratio<T>
where
    T: Clone + Integer,
{
    type Output = Ratio<T>;

    #[inline]
    fn inv(self) -> Ratio<T> {
        self.recip()
    }
}

// Constants
impl<T: ConstZero + ConstOne> Ratio<T> {
    /// A constant `Ratio` 0/1.
    pub const ZERO: Self = Self::new_raw(T::ZERO, T::ONE);
}

impl<T: Clone + Integer + ConstZero + ConstOne> ConstZero for Ratio<T> {
    const ZERO: Self = Self::ZERO;
}

impl<T: Clone + Integer> Zero for Ratio<T> {
    #[inline]
    fn zero() -> Ratio<T> {
        Ratio::new_raw(Zero::zero(), One::one())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.numer.is_zero()
    }

    #[inline]
    fn set_zero(&mut self) {
        self.numer.set_zero();
        self.denom.set_one();
    }
}

impl<T: ConstOne> Ratio<T> {
    /// A constant `Ratio` 1/1.
    pub const ONE: Self = Self::new_raw(T::ONE, T::ONE);
}

impl<T: Clone + Integer + ConstOne> ConstOne for Ratio<T> {
    const ONE: Self = Self::ONE;
}

impl<T: Clone + Integer> One for Ratio<T> {
    #[inline]
    fn one() -> Ratio<T> {
        Ratio::new_raw(One::one(), One::one())
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.numer == self.denom
    }

    #[inline]
    fn set_one(&mut self) {
        self.numer.set_one();
        self.denom.set_one();
    }
}

impl<T: Clone + Integer> Num for Ratio<T> {
    type FromStrRadixErr = ParseRatioError;

    /// Parses `numer/denom` where the numbers are in base `radix`.
    fn from_str_radix(s: &str, radix: u32) -> Result<Ratio<T>, ParseRatioError> {
        if s.splitn(2, '/').count() == 2 {
            let mut parts = s.splitn(2, '/').map(|ss| {
                T::from_str_radix(ss, radix).map_err(|_| ParseRatioError {
                    kind: RatioErrorKind::ParseError,
                })
            });
            let numer: T = parts.next().unwrap()?;
            let denom: T = parts.next().unwrap()?;
            if denom.is_zero() {
                Err(ParseRatioError {
                    kind: RatioErrorKind::ZeroDenominator,
                })
            } else {
                Ok(Ratio::new(numer, denom))
            }
        } else {
            Err(ParseRatioError {
                kind: RatioErrorKind::ParseError,
            })
        }
    }
}

impl<T: Clone + Integer + Signed> Signed for Ratio<T> {
    #[inline]
    fn abs(&self) -> Ratio<T> {
        if self.is_negative() {
            -self.clone()
        } else {
            self.clone()
        }
    }

    #[inline]
    fn abs_sub(&self, other: &Ratio<T>) -> Ratio<T> {
        if *self <= *other {
            Zero::zero()
        } else {
            self - other
        }
    }

    #[inline]
    fn signum(&self) -> Ratio<T> {
        if self.is_positive() {
            Self::one()
        } else if self.is_zero() {
            Self::zero()
        } else {
            -Self::one()
        }
    }

    #[inline]
    fn is_positive(&self) -> bool {
        (self.numer.is_positive() && self.denom.is_positive())
            || (self.numer.is_negative() && self.denom.is_negative())
    }

    #[inline]
    fn is_negative(&self) -> bool {
        (self.numer.is_negative() && self.denom.is_positive())
            || (self.numer.is_positive() && self.denom.is_negative())
    }
}

// String conversions
macro_rules! impl_formatting {
    ($fmt_trait:ident, $prefix:expr, $fmt_str:expr, $fmt_alt:expr) => {
        impl<T: $fmt_trait + Clone + Integer> $fmt_trait for Ratio<T> {
            #[cfg(feature = "std")]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                let pre_pad = if self.denom.is_one() {
                    format!($fmt_str, self.numer)
                } else {
                    if f.alternate() {
                        format!(concat!($fmt_str, "/", $fmt_alt), self.numer, self.denom)
                    } else {
                        format!(concat!($fmt_str, "/", $fmt_str), self.numer, self.denom)
                    }
                };
                if let Some(pre_pad) = pre_pad.strip_prefix("-") {
                    f.pad_integral(false, $prefix, pre_pad)
                } else {
                    f.pad_integral(true, $prefix, &pre_pad)
                }
            }
            #[cfg(not(feature = "std"))]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                let plus = if f.sign_plus() && self.numer >= T::zero() {
                    "+"
                } else {
                    ""
                };
                if self.denom.is_one() {
                    if f.alternate() {
                        write!(f, concat!("{}", $fmt_alt), plus, self.numer)
                    } else {
                        write!(f, concat!("{}", $fmt_str), plus, self.numer)
                    }
                } else {
                    if f.alternate() {
                        write!(
                            f,
                            concat!("{}", $fmt_alt, "/", $fmt_alt),
                            plus, self.numer, self.denom
                        )
                    } else {
                        write!(
                            f,
                            concat!("{}", $fmt_str, "/", $fmt_str),
                            plus, self.numer, self.denom
                        )
                    }
                }
            }
        }
    };
}

impl_formatting!(Display, "", "{}", "{:#}");
impl_formatting!(Octal, "0o", "{:o}", "{:#o}");
impl_formatting!(Binary, "0b", "{:b}", "{:#b}");
impl_formatting!(LowerHex, "0x", "{:x}", "{:#x}");
impl_formatting!(UpperHex, "0x", "{:X}", "{:#X}");
impl_formatting!(LowerExp, "", "{:e}", "{:#e}");
impl_formatting!(UpperExp, "", "{:E}", "{:#E}");

impl<T: FromStr + Clone + Integer> FromStr for Ratio<T> {
    type Err = ParseRatioError;

    /// Parses `numer/denom` or just `numer`.
    fn from_str(s: &str) -> Result<Ratio<T>, ParseRatioError> {
        let mut split = s.splitn(2, '/');

        let n = split.next().ok_or(ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;
        let num = FromStr::from_str(n).map_err(|_| ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;

        let d = split.next().unwrap_or("1");
        let den = FromStr::from_str(d).map_err(|_| ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;

        if Zero::is_zero(&den) {
            Err(ParseRatioError {
                kind: RatioErrorKind::ZeroDenominator,
            })
        } else {
            Ok(Ratio::new(num, den))
        }
    }
}

impl<T> From<Ratio<T>> for (T, T) {
    fn from(val: Ratio<T>) -> Self {
        (val.numer, val.denom)
    }
}

#[cfg(feature = "serialization")]
impl<T> data_serialization::Serialize for Ratio<T>
where
    T: data_serialization::Serialize + Clone + Integer + PartialOrd,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        (self.numer(), self.denom()).serialize(serializer)
    }
}

#[cfg(feature = "serialization")]
impl<'de, T> data_serialization::Deserialize<'de> for Ratio<T>
where
    T: data_serialization::Deserialize<'de> + Clone + Integer + PartialOrd,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: data_serialization::Deserializer<'de>,
    {
        use data_serialization::de::Error;
        use data_serialization::de::Unexpected;
        let (numer, denom): (T, T) = data_serialization::Deserialize::deserialize(deserializer)?;
        if denom.is_zero() {
            Err(Error::invalid_value(
                Unexpected::Signed(0),
                &"a ratio with non-zero denominator",
            ))
        } else {
            Ok(Ratio::new_raw(numer, denom))
        }
    }
}

// FIXME: Bubble up specific errors
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParseRatioError {
    kind: RatioErrorKind,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum RatioErrorKind {
    ParseError,
    ZeroDenominator,
}

impl fmt::Display for ParseRatioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.description().fmt(f)
    }
}

#[cfg(feature = "std")]
impl Error for ParseRatioError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.kind.description()
    }
}

impl RatioErrorKind {
    fn description(&self) -> &'static str {
        match *self {
            RatioErrorKind::ParseError => "failed to parse integer",
            RatioErrorKind::ZeroDenominator => "zero value denominator",
        }
    }
}

#[cfg(feature = "bigint")]
impl FromPrimitive for Ratio<BigInt> {
    fn from_i64(n: i64) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_i128(n: i128) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_u128(n: u128) -> Option<Self> {
        Some(Ratio::from_integer(n.into()))
    }

    fn from_f32(n: f32) -> Option<Self> {
        Ratio::from_float(n)
    }

    fn from_f64(n: f64) -> Option<Self> {
        Ratio::from_float(n)
    }
}

macro_rules! from_primitive_integer {
    ($typ:ty, $approx:ident) => {
        impl FromPrimitive for Ratio<$typ> {
            fn from_i64(n: i64) -> Option<Self> {
                <$typ as FromPrimitive>::from_i64(n).map(Ratio::from_integer)
            }

            fn from_i128(n: i128) -> Option<Self> {
                <$typ as FromPrimitive>::from_i128(n).map(Ratio::from_integer)
            }

            fn from_u64(n: u64) -> Option<Self> {
                <$typ as FromPrimitive>::from_u64(n).map(Ratio::from_integer)
            }

            fn from_u128(n: u128) -> Option<Self> {
                <$typ as FromPrimitive>::from_u128(n).map(Ratio::from_integer)
            }

            fn from_f32(n: f32) -> Option<Self> {
                $approx(n, 10e-20, 30)
            }

            fn from_f64(n: f64) -> Option<Self> {
                $approx(n, 10e-20, 30)
            }
        }
    };
}

from_primitive_integer!(i8, approximate_float);
from_primitive_integer!(i16, approximate_float);
from_primitive_integer!(i32, approximate_float);
from_primitive_integer!(i64, approximate_float);
from_primitive_integer!(i128, approximate_float);
from_primitive_integer!(isize, approximate_float);

from_primitive_integer!(u8, approximate_float_unsigned);
from_primitive_integer!(u16, approximate_float_unsigned);
from_primitive_integer!(u32, approximate_float_unsigned);
from_primitive_integer!(u64, approximate_float_unsigned);
from_primitive_integer!(u128, approximate_float_unsigned);
from_primitive_integer!(usize, approximate_float_unsigned);

impl<T: Integer + Signed + Bounded + NumCast + Clone> Ratio<T> {
    pub fn approximate_float<F: FloatCore + NumCast>(f: F) -> Option<Ratio<T>> {
        // 1/10e-20 < 1/2**32 which seems like a good default, and 30 seems
        // to work well. Might want to choose something based on the types in the future, e.g.
        // T::max().recip() and T::bits() or something similar.
        let epsilon = <F as NumCast>::from(10e-20).expect("Can't convert 10e-20");
        approximate_float(f, epsilon, 30)
    }
}

impl<T: Integer + Unsigned + Bounded + NumCast + Clone> Ratio<T> {
    pub fn approximate_float_unsigned<F: FloatCore + NumCast>(f: F) -> Option<Ratio<T>> {
        // 1/10e-20 < 1/2**32 which seems like a good default, and 30 seems
        // to work well. Might want to choose something based on the types in the future, e.g.
        // T::max().recip() and T::bits() or something similar.
        let epsilon = <F as NumCast>::from(10e-20).expect("Can't convert 10e-20");
        approximate_float_unsigned(f, epsilon, 30)
    }
}

fn approximate_float<T, F>(val: F, max_error: F, max_iterations: usize) -> Option<Ratio<T>>
where
    T: Integer + Signed + Bounded + NumCast + Clone,
    F: FloatCore + NumCast,
{
    let negative = val.is_sign_negative();
    let abs_val = val.abs();

    let r = approximate_float_unsigned(abs_val, max_error, max_iterations)?;

    // Make negative again if needed
    Some(if negative { r.neg() } else { r })
}

// No Unsigned constraint because this also works on positive integers and is called
// like that, see above
fn approximate_float_unsigned<T, F>(val: F, max_error: F, max_iterations: usize) -> Option<Ratio<T>>
where
    T: Integer + Bounded + NumCast + Clone,
    F: FloatCore + NumCast,
{
    // Continued fractions algorithm
    // https://web.archive.org/web/20200629111319/http://mathforum.org:80/dr.math/faq/faq.fractions.html#decfrac

    if val < F::zero() || val.is_nan() {
        return None;
    }

    let mut q = val;
    let mut n0 = T::zero();
    let mut d0 = T::one();
    let mut n1 = T::one();
    let mut d1 = T::zero();

    let t_max = T::max_value();
    let t_max_f = <F as NumCast>::from(t_max.clone())?;

    // 1/epsilon > T::MAX
    let epsilon = t_max_f.recip();

    // Overflow
    if q > t_max_f {
        return None;
    }

    for _ in 0..max_iterations {
        let a = match <T as NumCast>::from(q) {
            None => break,
            Some(a) => a,
        };

        let a_f = match <F as NumCast>::from(a.clone()) {
            None => break,
            Some(a_f) => a_f,
        };
        let f = q - a_f;

        // Prevent overflow
        if !a.is_zero()
            && (n1 > t_max.clone() / a.clone()
                || d1 > t_max.clone() / a.clone()
                || a.clone() * n1.clone() > t_max.clone() - n0.clone()
                || a.clone() * d1.clone() > t_max.clone() - d0.clone())
        {
            break;
        }

        let n = a.clone() * n1.clone() + n0.clone();
        let d = a.clone() * d1.clone() + d0.clone();

        n0 = n1;
        d0 = d1;
        n1 = n.clone();
        d1 = d.clone();

        // Simplify fraction. Doing so here instead of at the end
        // allows us to get closer to the target value without overflows
        let g = Integer::gcd(&n1, &d1);
        if !g.is_zero() {
            n1 = n1 / g.clone();
            d1 = d1 / g.clone();
        }

        // Close enough?
        let (n_f, d_f) = match (<F as NumCast>::from(n), <F as NumCast>::from(d)) {
            (Some(n_f), Some(d_f)) => (n_f, d_f),
            _ => break,
        };
        if (n_f / d_f - val).abs() < max_error {
            break;
        }

        // Prevent division by ~0
        if f < epsilon {
            break;
        }
        q = f.recip();
    }

    // Overflow
    if d1.is_zero() {
        return None;
    }

    Some(Ratio::new(n1, d1))
}

#[cfg(not(feature = "bigint"))]
macro_rules! to_primitive_small {
    ($($type_name:ty)*) => ($(
        impl ToPrimitive for Ratio<$type_name> {
            fn to_i64(&self) -> Option<i64> {
                self.to_integer().to_i64()
            }

            fn to_i128(&self) -> Option<i128> {
                self.to_integer().to_i128()
            }

            fn to_u64(&self) -> Option<u64> {
                self.to_integer().to_u64()
            }

            fn to_u128(&self) -> Option<u128> {
                self.to_integer().to_u128()
            }

            fn to_f64(&self) -> Option<f64> {
                let float = self.numer.to_f64().unwrap() / self.denom.to_f64().unwrap();
                if float.is_nan() {
                    None
                } else {
                    Some(float)
                }
            }
        }
    )*)
}

#[cfg(not(feature = "bigint"))]
to_primitive_small!(u8 i8 u16 i16 u32 i32);

#[cfg(all(target_pointer_width = "32", not(feature = "bigint")))]
to_primitive_small!(usize isize);

#[cfg(not(feature = "bigint"))]
macro_rules! to_primitive_64 {
    ($($type_name:ty)*) => ($(
        impl ToPrimitive for Ratio<$type_name> {
            fn to_i64(&self) -> Option<i64> {
                self.to_integer().to_i64()
            }

            fn to_i128(&self) -> Option<i128> {
                self.to_integer().to_i128()
            }

            fn to_u64(&self) -> Option<u64> {
                self.to_integer().to_u64()
            }

            fn to_u128(&self) -> Option<u128> {
                self.to_integer().to_u128()
            }

            fn to_f64(&self) -> Option<f64> {
                let float = ratio_to_f64(
                    self.numer as i128,
                    self.denom as i128
                );
                if float.is_nan() {
                    None
                } else {
                    Some(float)
                }
            }
        }
    )*)
}

#[cfg(not(feature = "bigint"))]
to_primitive_64!(u64 i64);

#[cfg(all(target_pointer_width = "64", not(feature = "bigint")))]
to_primitive_64!(usize isize);

#[cfg(feature = "bigint")]
impl<T: Clone + Integer + ToPrimitive + ToBigInt> ToPrimitive for Ratio<T> {
    fn to_i64(&self) -> Option<i64> {
        self.to_integer().to_i64()
    }

    fn to_i128(&self) -> Option<i128> {
        self.to_integer().to_i128()
    }

    fn to_u64(&self) -> Option<u64> {
        self.to_integer().to_u64()
    }

    fn to_u128(&self) -> Option<u128> {
        self.to_integer().to_u128()
    }

    fn to_f64(&self) -> Option<f64> {
        let float = match (self.numer.to_i64(), self.denom.to_i64()) {
            (Some(numer), Some(denom)) => ratio_to_f64(
                <i128 as From<_>>::from(numer),
                <i128 as From<_>>::from(denom),
            ),
            _ => {
                let numer: BigInt = self.numer.to_bigint()?;
                let denom: BigInt = self.denom.to_bigint()?;
                ratio_to_f64(numer, denom)
            }
        };
        if float.is_nan() {
            None
        } else {
            Some(float)
        }
    }
}

trait Bits {
    fn bits(&self) -> u64;
}

#[cfg(feature = "bigint")]
impl Bits for BigInt {
    fn bits(&self) -> u64 {
        self.bits()
    }
}

impl Bits for i128 {
    fn bits(&self) -> u64 {
        (128 - self.wrapping_abs().leading_zeros()).into()
    }
}

/// Converts a ratio of `T` to an f64.
///
/// In addition to stated trait bounds, `T` must be able to hold numbers 56 bits larger than
/// the largest of `numer` and `denom`. This is automatically true if `T` is `BigInt`.
fn ratio_to_f64<T: Bits + Clone + Integer + Signed + ShlAssign<usize> + ToPrimitive>(
    numer: T,
    denom: T,
) -> f64 {
    assert_eq!(
        f64::RADIX, 2,
        "only floating point implementations with radix 2 are supported"
    );

    // Inclusive upper and lower bounds to the range of exactly-representable ints in an f64.
    const MAX_EXACT_INT: i64 = 1i64 << f64::MANTISSA_DIGITS;
    const MIN_EXACT_INT: i64 = -MAX_EXACT_INT;

    let flo_sign = numer.signum().to_f64().unwrap() / denom.signum().to_f64().unwrap();
    if !flo_sign.is_normal() {
        return flo_sign;
    }

    // Fast track: both sides can losslessly be converted to f64s. In this case, letting the
    // FPU do the job is faster and easier. In any other case, converting to f64s may lead
    // to an inexact result: https://stackoverflow.com/questions/56641441/.
    if let (Some(n), Some(d)) = (numer.to_i64(), denom.to_i64()) {
        let exact = MIN_EXACT_INT..=MAX_EXACT_INT;
        if exact.contains(&n) && exact.contains(&d) {
            return n.to_f64().unwrap() / d.to_f64().unwrap();
        }
    }

    // Otherwise, the goal is to obtain a quotient with at least 55 bits. 53 of these bits will
    // be used as the mantissa of the resulting float, and the remaining two are for rounding.
    // There's an error of up to 1 on the number of resulting bits, so we may get either 55 or
    // 56 bits.
    let mut numer = numer.abs();
    let mut denom = denom.abs();
    let (is_diff_positive, absolute_diff) = match numer.bits().checked_sub(denom.bits()) {
        Some(diff) => (true, diff),
        None => (false, denom.bits() - numer.bits()),
    };

    // Filter out overflows and underflows. After this step, the signed difference fits in an
    // isize.
    if is_diff_positive && absolute_diff > f64::MAX_EXP as u64 {
        return f64::INFINITY * flo_sign;
    }
    if !is_diff_positive && absolute_diff > -f64::MIN_EXP as u64 + f64::MANTISSA_DIGITS as u64 + 1 {
        return 0.0 * flo_sign;
    }
    let diff = if is_diff_positive {
        absolute_diff.to_isize().unwrap()
    } else {
        -absolute_diff.to_isize().unwrap()
    };

    // Shift is chosen so that the quotient will have 55 or 56 bits. The exception is if the
    // quotient is going to be subnormal, in which case it may have fewer bits.
    let shift: isize = diff.max(f64::MIN_EXP as isize) - f64::MANTISSA_DIGITS as isize - 2;
    if shift >= 0 {
        denom <<= shift as usize
    } else {
        numer <<= -shift as usize
    };

    let (quotient, remainder) = numer.div_rem(&denom);

    // This is guaranteed to fit since we've set up quotient to be at most 56 bits.
    let mut quotient = quotient.to_u64().unwrap();
    let n_rounding_bits = {
        let quotient_bits = 64 - quotient.leading_zeros() as isize;
        let subnormal_bits = f64::MIN_EXP as isize - shift;
        quotient_bits.max(subnormal_bits) - f64::MANTISSA_DIGITS as isize
    } as usize;
    debug_assert!(n_rounding_bits == 2 || n_rounding_bits == 3);
    let rounding_bit_mask = (1u64 << n_rounding_bits) - 1;

    // Round to 53 bits with round-to-even. For rounding, we need to take into account both
    // our rounding bits and the division's remainder.
    let ls_bit = quotient & (1u64 << n_rounding_bits) != 0;
    let ms_rounding_bit = quotient & (1u64 << (n_rounding_bits - 1)) != 0;
    let ls_rounding_bits = quotient & (rounding_bit_mask >> 1) != 0;
    if ms_rounding_bit && (ls_bit || ls_rounding_bits || !remainder.is_zero()) {
        quotient += 1u64 << n_rounding_bits;
    }
    quotient &= !rounding_bit_mask;

    // The quotient is guaranteed to be exactly representable as it's now 53 bits + 2 or 3
    // trailing zeros, so there is no risk of a rounding error here.
    let q_float = quotient as f64 * flo_sign;
    ldexp(q_float, shift as i32)
}

/// Multiply `x` by 2 to the power of `exp`. Returns an accurate result even if `2^exp` is not
/// representable.
fn ldexp(x: f64, exp: i32) -> f64 {
    assert_eq!(
        f64::RADIX, 2,
        "only floating point implementations with radix 2 are supported"
    );

    const EXPONENT_MASK: u64 = 0x7ff << 52;
    const MAX_UNSIGNED_EXPONENT: i32 = 0x7fe;
    const MIN_SUBNORMAL_POWER: i32 = f64::MANTISSA_DIGITS as i32;

    if x.is_zero() || x.is_infinite() || x.is_nan() {
        return x;
    }

    // Filter out obvious over / underflows to make sure the resulting exponent fits in an isize.
    if exp > 3 * f64::MAX_EXP {
        return f64::INFINITY * x.signum();
    } else if exp < -3 * f64::MAX_EXP {
        return 0.0 * x.signum();
    }

    // curr_exp is the x's *biased* exponent, and is in the [-54, MAX_UNSIGNED_EXPONENT] range.
    let (bits, curr_exp) = if !x.is_normal() {
        // If x is subnormal, we make it normal by multiplying by 2^53. This causes no loss of
        // precision or rounding.
        let normal_x = x * 2f64.powi(MIN_SUBNORMAL_POWER);
        let bits = normal_x.to_bits();
        // This cast is safe because the exponent is at most 0x7fe, which fits in an i32.
        (
            bits,
            ((bits & EXPONENT_MASK) >> 52) as i32 - MIN_SUBNORMAL_POWER,
        )
    } else {
        let bits = x.to_bits();
        let curr_exp = (bits & EXPONENT_MASK) >> 52;
        // This cast is safe because the exponent is at most 0x7fe, which fits in an i32.
        (bits, curr_exp as i32)
    };

    // The addition can't overflow because exponent is between 0 and 0x7fe, and exp is between
    // -2*f64::MAX_EXP and 2*f64::MAX_EXP.
    let new_exp = curr_exp + exp;

    if new_exp > MAX_UNSIGNED_EXPONENT {
        f64::INFINITY * x.signum()
    } else if new_exp > 0 {
        // Normal case: exponent is not too large nor subnormal.
        let new_bits = (bits & !EXPONENT_MASK) | ((new_exp as u64) << 52);
        f64::from_bits(new_bits)
    } else if new_exp >= -(f64::MANTISSA_DIGITS as i32) {
        // Result is subnormal but may not be zero.
        // In this case, we increase the exponent by 54 to make it normal, then multiply the end
        // result by 2^-53. This results in a single multiplication with no prior rounding error,
        // so there is no risk of double rounding.
        let new_exp = new_exp + MIN_SUBNORMAL_POWER;
        debug_assert!(new_exp >= 0);
        let new_bits = (bits & !EXPONENT_MASK) | ((new_exp as u64) << 52);
        f64::from_bits(new_bits) * 2f64.powi(-MIN_SUBNORMAL_POWER)
    } else {
        // Result is zero.
        return 0.0 * x.signum();
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
fn hash<T: Hash>(x: &T) -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    let mut hasher = <RandomState as BuildHasher>::Hasher::new();
    x.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/numeric/rational/unit/src/lib/test_1723.rs"));
