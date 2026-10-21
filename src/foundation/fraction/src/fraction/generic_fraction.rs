use crate::fraction::Sign;
use crate::{
    display, Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, ConstOne, ConstZero,
    FromPrimitive, Integer, Num, One, Ratio, Signed, ToPrimitive, Zero,
};
#[cfg(feature = "with-bigint")]
use crate::{BigInt, BigUint};
use error::ParseError;
use std::iter::{Product, Sum};

use generic::{read_generic_integer, GenericInteger};

use std::cmp::{Eq, Ordering, PartialEq, PartialOrd};
use std::hash::{Hash, Hasher};
use std::num::FpCategory;
use std::ops::{Add, Mul, Neg};

use std::fmt;
use std::str::FromStr;

/// Parses a non-negative integer component for a fraction text grammar.
///
/// `GenericFraction` stores the sign separately from its `Ratio`, so signs are
/// only valid on the complete input and not on individual numeric components.
/// Callers must validate that `radix` is in `2..=36` before calling this helper.
#[inline]
pub(super) fn parse_magnitude<T>(value: &str, radix: u32) -> Result<T, ParseError>
where
    T: Integer,
{
    if matches!(value.as_bytes().first(), Some(b'+' | b'-')) {
        return Err(ParseError::ParseIntError);
    }
    T::from_str_radix(value, radix).map_err(|_| ParseError::ParseIntError)
}

/// Generic implementation of the fraction type
///
/// # Examples
///
/// ```
/// use fraction::GenericFraction;
///
/// type F = GenericFraction<u8>;
///
/// let first = F::new (1u8, 2u8);
/// let second = F::new (2u8, 8u8);
///
/// assert_eq! (first + second, F::new (3u8, 4u8));
/// ```
///
///
/// Since GenericFraction keeps its sign explicitly and independently of the numerics,
/// it is not recommended to use signed types, although it's completely valid with the cost of target type capacity.
/// For numeric-component forms, text parsers accept at most one optional sign
/// at the beginning of the complete input. Signs inside a numerator,
/// denominator, or other numeric component are rejected so that the stored
/// `Ratio` remains non-negative.
///
/// ```
/// use fraction::GenericFraction;
///
/// type F = GenericFraction<i8>;
///
/// let first = F::new (1, 2);
/// let second = F::new (2, 8);
///
/// assert_eq! (first + second, F::new (3, 4));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "with-serialization-support", derive(Serialize, Deserialize))]
pub enum GenericFraction<T>
where
    T: Clone + Integer,
{
    Rational(Sign, Ratio<T>),
    Infinity(Sign),
    NaN,
}

/// Copy semantics to be applied for the target type, but only if T also has it.
impl<T> Copy for GenericFraction<T> where T: Copy + Integer {}

impl<T> Default for GenericFraction<T>
where
    T: Clone + Integer,
{
    fn default() -> Self {
        Self::zero()
    }
}

impl<T> FromStr for GenericFraction<T>
where
    T: Clone + Integer + CheckedAdd + CheckedMul,
{
    type Err = ParseError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let (sign, start) = match src.as_bytes().first() {
            Some(b'-') => (Sign::Minus, 1),
            Some(b'+') => (Sign::Plus, 1),
            _ => (Sign::Plus, 0),
        };
        let mut dot = None;
        let mut slash = None;
        for (idx, byte) in src.as_bytes()[start..].iter().enumerate() {
            match *byte {
                b'+' | b'-' => return Err(ParseError::ParseIntError),
                b'.' if dot.is_none() => dot = Some(idx + start),
                b'/' if slash.is_none() => slash = Some(idx + start),
                _ => {}
            }
        }
        // The scan above already rejects component-local signs, so these hot-path
        // conversions deliberately avoid repeating parse_magnitude's sign check.
        // A dot takes precedence over a slash: `1/2.5` therefore follows the
        // decimal arm and is rejected when its whole component is parsed.
        match (dot, slash) {
            (None, Some(split_idx)) => {
                let num = match T::from_str_radix(&src[start..split_idx], 10) {
                    Ok(value) => value,
                    Err(_) => return Err(ParseError::ParseIntError),
                };
                let den = match T::from_str_radix(&src[split_idx + 1..], 10) {
                    Ok(value) => value,
                    Err(_) => return Err(ParseError::ParseIntError),
                };

                Ok(Self::new_signed(sign, num, den))
            }
            (Some(split_idx), _) => {
                let who = &src[start..split_idx];

                let mut num = match T::from_str_radix(who, 10) {
                    Ok(value) => value,
                    Err(_) => return Err(ParseError::ParseIntError),
                };

                // skip trailing zeros
                let len = src[split_idx + 1..].trim_end_matches('0').len();
                let fra = if len > 0 {
                    match T::from_str_radix(&src[split_idx + 1..split_idx + 1 + len], 10) {
                        Ok(value) => value,
                        Err(_) => return Err(ParseError::ParseIntError),
                    }
                } else {
                    T::zero()
                };

                let mut den = T::one();

                if len > 0 {
                    let mut t10 = T::one();
                    for _ in 0..9 {
                        t10 = if let Some(t10) = t10.checked_add(&den) {
                            t10
                        } else {
                            return Err(ParseError::OverflowError);
                        };
                    }

                    for _ in 0..len {
                        num = if let Some(num) = num.checked_mul(&t10) {
                            num
                        } else {
                            return Err(ParseError::OverflowError);
                        };
                        den = if let Some(den) = den.checked_mul(&t10) {
                            den
                        } else {
                            return Err(ParseError::OverflowError);
                        };
                    }
                }

                let num = if let Some(num) = num.checked_add(&fra) {
                    num
                } else {
                    return Err(ParseError::OverflowError);
                };

                Ok(Self::new_signed(sign, num, den))
            }
            (None, None) => {
                let num = match T::from_str_radix(&src[start..], 10) {
                    Ok(value) => value,
                    Err(_) => return Err(ParseError::ParseIntError),
                };

                Ok(Self::new_signed(sign, num, T::one()))
            }
        }
    }
}

impl<T> GenericFraction<T>
where
    T: Clone + Integer,
{
    /// Constructs a new fraction with the specified numerator and denominator
    /// Handles gracefully signed integers even if the storage type is unsigned and vise versa
    /// The arguments can be of any integer types imlementing the necessary traits
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::{GenericFraction, Sign};
    /// type F = GenericFraction<u16>;
    ///
    /// let f12 = F::new_generic(Sign::Plus, 1i8, 2u8).unwrap();
    /// let f34 = F::new_generic(Sign::Plus, 3i16, 4u32).unwrap();
    /// let f56 = F::new_generic(Sign::Plus, 5i64, 6u128).unwrap();
    /// let f78 = F::new_generic(Sign::Plus, 7usize, 8isize).unwrap();
    ///
    /// assert_eq! ((*f12.numer().unwrap(), *f12.denom().unwrap()), (1u16, 2u16));
    /// assert_eq! ((*f34.numer().unwrap(), *f34.denom().unwrap()), (3u16, 4u16));
    /// assert_eq! ((*f56.numer().unwrap(), *f56.denom().unwrap()), (5u16, 6u16));
    /// assert_eq! ((*f78.numer().unwrap(), *f78.denom().unwrap()), (7u16, 8u16));
    /// ````
    pub fn new_generic<N, D>(sign: Sign, num: N, den: D) -> Option<GenericFraction<T>>
    where
        N: GenericInteger + PartialOrd,
        D: GenericInteger + PartialOrd,
        T: GenericInteger,
    {
        let (ns, num): (Sign, T) = read_generic_integer(num)?;
        let (ds, den): (Sign, T) = read_generic_integer(den)?;

        let sign = ns * ds * sign;

        Some(Self::new_signed(sign, num, den))
    }

    pub(super) fn new_signed<N, D>(sign: Sign, num: N, den: D) -> GenericFraction<T>
    where
        N: Into<T>,
        D: Into<T>,
    {
        let num: T = num.into();
        let den: T = den.into();

        if den.is_zero() {
            if num.is_zero() {
                GenericFraction::NaN
            } else {
                GenericFraction::Infinity(sign)
            }
        } else {
            GenericFraction::Rational(sign, Ratio::new(num, den))
        }
    }

    /// Constructs a new fraction with the specified numerator and denominator
    ///
    /// The arguments must me either of `T` type, or implement `Into<T>` trait.
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u16>;
    ///
    /// let _f = F::new(1u8, 2u16);
    /// ```
    pub fn new<N, D>(num: N, den: D) -> GenericFraction<T>
    where
        N: Into<T>,
        D: Into<T>,
    {
        Self::new_signed(Sign::Plus, num, den)
    }

    /// Constructs a new negative fraction with the specified numerator and denominator
    ///
    /// The arguments must be either of `T` type, or implement `Into<T>` trait.
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u16>;
    ///
    /// let _f = F::new_neg (1u8, 2u16);
    /// ```
    pub fn new_neg<N, D>(num: N, den: D) -> GenericFraction<T>
    where
        N: Into<T>,
        D: Into<T>,
    {
        Self::new_signed(Sign::Minus, num, den)
    }

    /// Constructs a new fraction without types casting, checking for denom == 0 and reducing numbers.
    ///
    /// You must be careful with this function because all the other functionality parts rely on the
    /// numbers to be reduced. That said, in the normal case 2/4 has to be reduced to 1/2, but it will not
    /// happen with new_raw.
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// let _f = F::new_raw (1u8, 2u8);
    /// ```
    pub const fn new_raw(num: T, den: T) -> GenericFraction<T> {
        GenericFraction::Rational(Sign::Plus, Ratio::new_raw(num, den))
    }

    /// The same as [fn new_raw](enum.GenericFraction.html#method.new_raw), but allows explicitly set sign.
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::{GenericFraction, Sign};
    /// type F = GenericFraction<u8>;
    ///
    /// let _f = F::new_raw_signed(Sign::Minus, 1u8, 2u8);
    /// ```
    pub const fn new_raw_signed(sign: Sign, num: T, den: T) -> GenericFraction<T> {
        GenericFraction::Rational(sign, Ratio::new_raw(num, den))
    }

    /// Returns a reference to the numerator value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// let fra = F::new (5u8, 6u8);
    /// assert_eq! (5, *fra.numer ().unwrap ());
    /// ```
    pub const fn numer(&self) -> Option<&T> {
        match *self {
            GenericFraction::Rational(_, ref r) => Some(r.numer()),
            _ => None,
        }
    }

    /// Returns a reference to the denominator value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// let fra = F::new (5u8, 6u8);
    /// assert_eq! (6, *fra.denom ().unwrap ());
    /// ```
    pub const fn denom(&self) -> Option<&T> {
        match *self {
            GenericFraction::Rational(_, ref r) => Some(r.denom()),
            _ => None,
        }
    }

    /// Returns a reference to the sign value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::{ GenericFraction, Sign };
    /// type F = GenericFraction<u8>;
    ///
    ///
    /// let fra = F::new (5u8, 6u8);
    /// assert_eq! (Sign::Plus, fra.sign ().unwrap ());
    ///
    /// let fra = F::new_neg (5u8, 6u8);
    /// assert_eq! (Sign::Minus, fra.sign ().unwrap ());
    ///
    ///
    /// let fra = F::infinity ();
    /// assert_eq! (Sign::Plus, fra.sign ().unwrap ());
    ///
    /// let fra = F::neg_infinity ();
    /// assert_eq! (Sign::Minus, fra.sign ().unwrap ());
    ///
    ///
    /// let fra = F::nan ();
    /// assert_eq! (None, fra.sign ());
    /// ```
    pub const fn sign(&self) -> Option<Sign> {
        match self {
            GenericFraction::Rational(s, _) => Some(*s),
            GenericFraction::Infinity(s) => Some(*s),
            _ => None,
        }
    }

    /// Generates a `GenericFraction<T>` from `GenericFraction<F>`
    /// where `T: From<F>`
    ///
    /// ```
    /// use fraction::{ Fraction, GenericFraction };
    /// type F8 = GenericFraction<u8>;
    ///
    /// let fra8 = F8::new (5u8, 6u8);
    /// assert_eq! (Fraction::new (5u64, 6u64), Fraction::from_fraction(fra8));
    /// ```
    pub fn from_fraction<F>(from: GenericFraction<F>) -> GenericFraction<T>
    where
        T: From<F>,
        F: Clone + Integer,
    {
        match from {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(sign) => GenericFraction::Infinity(sign),
            GenericFraction::Rational(sign, ratio) => {
                let (num, den): (F, F) = ratio.into();
                GenericFraction::Rational(sign, Ratio::new_raw(T::from(num), T::from(den)))
            }
        }
    }

    /// Generates a `GenericFraction<I>` from `GenericFraction<T>`
    /// where `T: Into<I>`
    ///
    /// ```
    /// use fraction::{ Fraction, GenericFraction };
    /// type F8 = GenericFraction<u8>;
    ///
    /// let fra8 = F8::new (5u8, 6u8);
    /// assert_eq! (Fraction::new (5u64, 6u64), fra8.into_fraction());
    /// ```
    pub fn into_fraction<I>(self) -> GenericFraction<I>
    where
        T: Into<I>,
        I: Clone + Integer,
    {
        match self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(sign) => GenericFraction::Infinity(sign),
            GenericFraction::Rational(sign, ratio) => {
                let (num, den): (T, T) = ratio.into();
                GenericFraction::Rational(sign, Ratio::new_raw(num.into(), den.into()))
            }
        }
    }
}

impl<T: Bounded + Clone + Integer> Bounded for GenericFraction<T> {
    fn min_value() -> Self {
        let one = T::one();
        let max = T::max_value();

        GenericFraction::Rational(Sign::Minus, Ratio::new(max, one))
    }

    fn max_value() -> Self {
        let one = T::one();
        let max = T::max_value();

        GenericFraction::Rational(Sign::Plus, Ratio::new(max, one))
    }
}

impl<T: Clone + Integer> PartialEq for GenericFraction<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (GenericFraction::NaN, GenericFraction::NaN) => true,
            (GenericFraction::Infinity(sign), GenericFraction::Infinity(osign)) => sign == osign,
            (
                GenericFraction::Rational(ref ls, ref l),
                GenericFraction::Rational(ref rs, ref r),
            ) => {
                if ls == rs {
                    l.eq(r)
                } else {
                    l.is_zero() && r.is_zero()
                }
            }
            _ => false,
        }
    }
}

impl<T: Clone + Integer + Hash> Hash for GenericFraction<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            GenericFraction::NaN => state.write_u8(0u8),
            GenericFraction::Infinity(sign) => {
                if let Sign::Plus = sign {
                    state.write_u8(1u8)
                } else {
                    state.write_u8(2u8)
                }
            }
            GenericFraction::Rational(sign, ratio) => {
                if *sign == Sign::Plus || ratio.is_zero() {
                    state.write_u8(3u8);
                } else {
                    state.write_u8(4u8);
                }

                ratio.hash(state);
            }
        }
    }
}

impl<T: Clone + Integer> Eq for GenericFraction<T> {}

impl<T: Clone + Integer> PartialOrd for GenericFraction<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Clone + Integer> Ord for GenericFraction<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (GenericFraction::NaN, GenericFraction::NaN) => Ordering::Equal,
            (GenericFraction::NaN, _) => Ordering::Less,
            (_, GenericFraction::NaN) => Ordering::Greater,
            (GenericFraction::Infinity(sign), GenericFraction::Infinity(osign)) => sign.cmp(osign),
            (GenericFraction::Infinity(Sign::Plus), GenericFraction::Rational(_, _)) => {
                Ordering::Greater
            }
            (GenericFraction::Infinity(Sign::Minus), GenericFraction::Rational(_, _)) => {
                Ordering::Less
            }
            (GenericFraction::Rational(_, _), GenericFraction::Infinity(Sign::Plus)) => {
                Ordering::Less
            }
            (GenericFraction::Rational(_, _), GenericFraction::Infinity(Sign::Minus)) => {
                Ordering::Greater
            }
            (
                GenericFraction::Rational(ref ls, ref l),
                GenericFraction::Rational(ref rs, ref r),
            ) => {
                if ls == rs {
                    match *ls {
                        Sign::Plus => l.cmp(r),
                        Sign::Minus => r.cmp(l),
                    }
                } else if l.is_zero() && r.is_zero() {
                    Ordering::Equal
                } else if *ls == Sign::Minus {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}

impl<T: Clone + Integer> Neg for GenericFraction<T> {
    type Output = GenericFraction<T>;

    fn neg(self) -> Self {
        match self {
            GenericFraction::NaN => self,
            GenericFraction::Infinity(sign) => GenericFraction::Infinity(-sign),
            GenericFraction::Rational(s, r) => {
                if r.is_zero() {
                    GenericFraction::Rational(Sign::Plus, r)
                } else {
                    GenericFraction::Rational(s.neg(), r)
                }
            }
        }
    }
}

impl<T: Clone + Integer> Neg for &GenericFraction<T> {
    type Output = GenericFraction<T>;

    fn neg(self) -> Self::Output {
        match *self {
            GenericFraction::NaN => GenericFraction::nan(),
            GenericFraction::Infinity(sign) => GenericFraction::Infinity(-sign),
            GenericFraction::Rational(s, ref r) => {
                if r.is_zero() {
                    GenericFraction::Rational(Sign::Plus, r.clone())
                } else {
                    GenericFraction::Rational(-s, r.clone())
                }
            }
        }
    }
}

impl<T: Clone + Integer> Sum for GenericFraction<T> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(GenericFraction::<T>::zero(), Add::add)
    }
}

impl<'a, T: 'a + Clone + Integer> Sum<&'a GenericFraction<T>> for GenericFraction<T> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        #[allow(clippy::redundant_closure)]
        iter.fold(GenericFraction::<T>::zero(), |ref s, x| Add::add(s, x))
    }
}

impl<T: Clone + Integer> Product for GenericFraction<T> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(GenericFraction::<T>::one(), Mul::mul)
    }
}

impl<'a, T: 'a + Clone + Integer> Product<&'a GenericFraction<T>> for GenericFraction<T> {
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        #[allow(clippy::redundant_closure)]
        iter.fold(GenericFraction::<T>::one(), |ref s, x| Mul::mul(s, x))
    }
}

impl<T: Clone + Integer> Zero for GenericFraction<T> {
    fn zero() -> Self {
        GenericFraction::Rational(Sign::Plus, Ratio::zero())
    }

    fn is_zero(&self) -> bool {
        match *self {
            GenericFraction::Rational(_, ref r) => r.is_zero(),
            _ => false,
        }
    }
}

impl<T: ConstOne + ConstZero + Integer + Clone> ConstZero for GenericFraction<T> {
    const ZERO: GenericFraction<T> =
        GenericFraction::Rational(Sign::Plus, Ratio::new_raw(ConstZero::ZERO, ConstOne::ONE));
}

impl<T: Clone + Integer> One for GenericFraction<T> {
    fn one() -> Self {
        GenericFraction::Rational(Sign::Plus, Ratio::one())
    }
}

impl<T: ConstOne + Integer + Clone> ConstOne for GenericFraction<T> {
    const ONE: GenericFraction<T> =
        GenericFraction::Rational(Sign::Plus, Ratio::new_raw(ConstOne::ONE, ConstOne::ONE));
}

impl<T: Clone + Integer> Num for GenericFraction<T> {
    /// Parsing errors use the crate's [`ParseError`] type.
    type FromStrRadixErr = ParseError;

    /// Parses a fraction in `numerator/denominator` form using `radix`.
    ///
    /// The slash and both numeric components are mandatory. An optional `+`
    /// or `-` may prefix the complete value, but signs inside either numeric
    /// component are rejected. Bases from 2 through 36 are supported.
    ///
    /// # Errors
    ///
    /// [`ParseError::ParseIntError`] reports malformed or unrepresentable
    /// components, [`ParseError::UnsupportedBase`] reports a radix outside
    /// that range, and [`ParseError::ZeroDenominator`] reports a zero
    /// denominator.
    ///
    /// ```
    /// use fraction::{error::ParseError, GenericFraction, Num};
    ///
    /// assert_eq!(
    ///     GenericFraction::<u8>::from_str_radix("101/10", 2).unwrap(),
    ///     GenericFraction::new(5, 2),
    /// );
    /// assert!(matches!(
    ///     GenericFraction::<u8>::from_str_radix("1/+10", 10),
    ///     Err(ParseError::ParseIntError),
    /// ));
    /// assert!(matches!(
    ///     GenericFraction::<u8>::from_str_radix("1/2", 1),
    ///     Err(ParseError::UnsupportedBase),
    /// ));
    /// assert!(matches!(
    ///     GenericFraction::<u8>::from_str_radix("1/0", 10),
    ///     Err(ParseError::ZeroDenominator),
    /// ));
    /// ```
    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        if !(2..=36).contains(&radix) {
            return Err(ParseError::UnsupportedBase);
        }
        let (sign, input) = if let Some(rem) = str.strip_prefix('-') {
            (Sign::Minus, rem)
        } else if let Some(rem) = str.strip_prefix('+') {
            (Sign::Plus, rem)
        } else {
            (Sign::Plus, str)
        };
        let (numerator, denominator) = input.split_once('/').ok_or(ParseError::ParseIntError)?;
        let numerator = parse_magnitude::<T>(numerator, radix)?;
        let denominator = parse_magnitude::<T>(denominator, radix)?;
        if denominator.is_zero() {
            Err(ParseError::ZeroDenominator)
        } else {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new(numerator, denominator),
            ))
        }
    }
}

impl<T: Clone + Integer> Signed for GenericFraction<T> {
    fn abs(&self) -> Self {
        GenericFraction::abs(self)
    }

    fn abs_sub(&self, other: &Self) -> Self {
        match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(sign) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(osign) => {
                    if sign == Sign::Minus || osign == Sign::Plus {
                        GenericFraction::zero()
                    } else {
                        GenericFraction::Infinity(Sign::Plus)
                    }
                }
                GenericFraction::Rational(_, _) => {
                    if sign == Sign::Plus {
                        GenericFraction::Infinity(sign)
                    } else {
                        GenericFraction::zero()
                    }
                }
            },
            GenericFraction::Rational(sign, ref l) => match *other {
                GenericFraction::NaN => GenericFraction::NaN,
                GenericFraction::Infinity(osign) => {
                    if osign == Sign::Plus {
                        GenericFraction::zero()
                    } else if sign == Sign::Minus {
                        GenericFraction::Infinity(Sign::Minus)
                    } else {
                        GenericFraction::Infinity(Sign::Plus)
                    }
                }
                GenericFraction::Rational(_, ref r) => {
                    if l < r {
                        GenericFraction::Rational(Sign::Plus, r - l)
                    } else {
                        GenericFraction::Rational(Sign::Plus, l - r)
                    }
                }
            },
        }
    }

    fn signum(&self) -> Self {
        GenericFraction::signum(self)
    }

    fn is_positive(&self) -> bool {
        GenericFraction::is_sign_positive(self)
    }

    fn is_negative(&self) -> bool {
        GenericFraction::is_sign_negative(self)
    }
}

impl<T: Clone + Integer + PartialEq + ToPrimitive> ToPrimitive for GenericFraction<T> {
    fn to_i64(&self) -> Option<i64> {
        match *self {
            GenericFraction::NaN => None,
            GenericFraction::Infinity(_) => None,
            #[allow(clippy::manual_filter)]
            GenericFraction::Rational(sign, ref r) if *r.denom() == T::one() => {
                if let Some(n) = r.numer().to_i64() {
                    if sign == Sign::Minus {
                        Some(-n)
                    } else {
                        Some(n)
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn to_u64(&self) -> Option<u64> {
        match *self {
            GenericFraction::NaN => None,
            GenericFraction::Infinity(_) => None,
            GenericFraction::Rational(sign, ref r) if *r.denom() == T::one() => {
                if sign == Sign::Minus {
                    None
                } else {
                    r.numer().to_u64()
                }
            }
            _ => None,
        }
    }

    fn to_f64(&self) -> Option<f64> {
        match *self {
            GenericFraction::NaN => Some(f64::NAN),
            GenericFraction::Infinity(sign) => Some(if sign == Sign::Minus {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }),
            GenericFraction::Rational(sign, ref r) => r
                .numer()
                .to_f64()
                .and_then(|n| r.denom().to_f64().map(|d| n / d))
                .map(|x| if sign == Sign::Minus { -x } else { x }),
        }
    }
}

impl<T: Clone + Integer> GenericFraction<T> {
    /// Returns NaN value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::nan (), F::new (0, 0));
    /// ```
    pub const fn nan() -> Self {
        GenericFraction::NaN
    }

    /// Returns positive Infinity value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::infinity (), F::new (1, 0));
    /// ```
    pub const fn infinity() -> Self {
        GenericFraction::Infinity(Sign::Plus)
    }

    /// Returns negative Infinity value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::neg_infinity (), F::new_neg (1, 0));
    /// ```
    pub const fn neg_infinity() -> Self {
        GenericFraction::Infinity(Sign::Minus)
    }

    /// Returns zero with negative sign
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::neg_zero (), F::new_neg (0, 1));
    /// ```
    pub fn neg_zero() -> Self {
        GenericFraction::Rational(Sign::Minus, Ratio::zero())
    }

    /// Returns minimal value greater than zero
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F8 = GenericFraction<u8>;
    /// type F16 = GenericFraction<u16>;
    ///
    /// assert_eq! (F8::min_positive_value (), F8::new (1u8, 255u8));
    /// assert_eq! (F16::min_positive_value (), F16::new (1u16, 65535u16));
    /// ```
    pub fn min_positive_value() -> Self
    where
        T: Bounded,
    {
        GenericFraction::Rational(Sign::Plus, Ratio::new(T::one(), T::max_value()))
    }

    /// Returns true if the value is NaN
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert! (F::nan ().is_nan ());
    /// assert! (F::new (0, 0).is_nan ());
    /// ```
    pub const fn is_nan(&self) -> bool {
        matches!(*self, GenericFraction::NaN)
    }

    /// Returns true if the value is Infinity (does not matter positive or negative)
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert! (F::infinity ().is_infinite ());
    /// assert! (F::new (1u8, 0).is_infinite ());
    /// assert! (F::new_neg (1u8, 0).is_infinite ());
    /// ```
    pub const fn is_infinite(&self) -> bool {
        matches!(*self, GenericFraction::Infinity(_))
    }

    /// Returns true if the value is not Infinity (does not matter positive or negative)
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert! (! F::infinity ().is_finite ());
    /// assert! (! F::new (1u8, 0).is_finite ());
    /// assert! (! F::new_neg (1u8, 0).is_finite ());
    /// ```
    pub const fn is_finite(&self) -> bool {
        !matches!(*self, GenericFraction::Infinity(_))
    }

    /// Returns true if the number is neither zero, Infinity or NaN
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert! (! F::nan ().is_normal ());
    /// assert! (! F::infinity ().is_normal ());
    /// assert! (! F::neg_infinity ().is_normal ());
    /// assert! (! F::new (0, 1u8).is_normal ());
    /// assert! (! F::neg_zero ().is_normal ());
    /// ```
    pub fn is_normal(&self) -> bool {
        match *self {
            GenericFraction::Rational(_, ref v) => !v.is_zero(),
            _ => false,
        }
    }

    /// Returns the floating point category of the number
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::FpCategory;
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::nan ().classify (), FpCategory::Nan);
    /// assert_eq! (F::infinity ().classify (), FpCategory::Infinite);
    /// assert_eq! (F::new (0, 1u8).classify (), FpCategory::Zero);
    /// assert_eq! (F::new (1u8, 1u8).classify (), FpCategory::Normal);
    /// ```
    pub fn classify(&self) -> FpCategory {
        match *self {
            GenericFraction::NaN => FpCategory::Nan,
            GenericFraction::Infinity(_) => FpCategory::Infinite,
            GenericFraction::Rational(_, ref v) if v.is_zero() => FpCategory::Zero,
            _ => FpCategory::Normal,
        }
    }

    /// Returns the largest integer less than or equal to the value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (7u8, 5u8).floor (), F::new (5u8, 5u8));
    /// assert_eq! (F::new_neg (4u8, 3u8).floor (), F::new_neg (2u8, 1u8));
    /// ```
    pub fn floor(&self) -> Self {
        match *self {
            GenericFraction::Rational(Sign::Plus, ref r) => {
                GenericFraction::Rational(Sign::Plus, r.floor())
            }
            GenericFraction::Rational(Sign::Minus, ref r) => {
                // Ceil of the unsigned ratio results in a floor for the signed fraction
                GenericFraction::Rational(Sign::Minus, r.ceil())
            }
            _ => self.clone(),
        }
    }

    /// Returns the smallest integer greater than or equal to the value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (7u8, 5u8).ceil (), F::new (10u8, 5u8));
    /// assert_eq! (F::new_neg (4u8, 3u8).ceil (), F::new_neg (1u8, 1u8));
    /// ```
    pub fn ceil(&self) -> Self {
        match *self {
            GenericFraction::Rational(Sign::Plus, ref r) => {
                GenericFraction::Rational(Sign::Plus, r.ceil())
            }
            GenericFraction::Rational(Sign::Minus, ref r) => {
                // Floor of the unsigned ratio results in a ceil for the signed fraction
                GenericFraction::Rational(Sign::Minus, r.floor())
            }
            _ => self.clone(),
        }
    }

    /// Returns the nearest integer to the value (.5 goes away from zero)
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (7u8, 5u8).round (), F::new (5u8, 5u8));
    /// assert_eq! (F::new (8u8, 5u8).round (), F::new (10u8, 5u8));
    /// assert_eq! (F::new (3u8, 2u8).round (), F::new (4u8, 2u8));
    /// assert_eq! (F::new (1u8, 2u8).round (), F::new (2u8, 2u8));
    /// assert_eq! (F::new_neg (3u8, 2u8).round (), F::new_neg (2u8, 1u8));
    /// ```
    pub fn round(&self) -> Self {
        match *self {
            GenericFraction::Rational(s, ref r) => GenericFraction::Rational(s, r.round()),
            _ => self.clone(),
        }
    }

    /// Returns the integer part of the value
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (7u8, 5u8).trunc (), F::new (5u8, 5u8));
    /// assert_eq! (F::new (8u8, 5u8).trunc (), F::new (5u8, 5u8));
    /// ```
    pub fn trunc(&self) -> Self {
        match *self {
            GenericFraction::Rational(s, ref r) => GenericFraction::Rational(s, r.trunc()),
            _ => self.clone(),
        }
    }

    /// Returns the fractional part of a number
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (7u8, 5u8).fract (), F::new (2u8, 5u8));
    /// assert_eq! (F::new (8u8, 5u8).fract (), F::new (3u8, 5u8));
    /// ```
    pub fn fract(&self) -> Self {
        match *self {
            GenericFraction::Rational(s, ref r) => GenericFraction::Rational(s, r.fract()),
            _ => GenericFraction::NaN,
        }
    }

    /// Returns the absolute value of self
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::nan ().abs (), F::nan ());
    /// assert_eq! (F::infinity ().abs (), F::infinity ());
    /// assert_eq! (F::neg_infinity ().abs (), F::infinity ());
    /// assert_eq! (F::new (1u8, 2u8).abs (), F::new (1u8, 2u8));
    /// assert_eq! (F::new_neg (1u8, 2u8).abs (), F::new (1u8, 2u8));
    /// ```
    pub fn abs(&self) -> Self {
        match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(_) => GenericFraction::Infinity(Sign::Plus),
            GenericFraction::Rational(_, ref r) => GenericFraction::Rational(Sign::Plus, r.clone()),
        }
    }

    /// Returns a number that represents the sign of self
    ///
    ///  * 1.0 if the number is positive, +0.0 or INFINITY
    ///  * -1.0 if the number is negative, -0.0 or NEG_INFINITY
    ///  * NAN if the number is NAN
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (1u8, 2u8).signum (), F::new (1u8, 1u8));
    /// assert_eq! (F::new (0u8, 1u8).signum (), F::new (1u8, 1u8));
    /// assert_eq! (F::infinity ().signum (), F::new (1u8, 1u8));
    /// assert_eq! (F::new_neg (1u8, 2u8).signum (), F::new_neg (1u8, 1u8));
    /// assert_eq! (F::neg_zero ().signum (), F::new_neg (1u8, 1u8));
    /// assert_eq! (F::neg_infinity ().signum (), F::new_neg (1u8, 1u8));
    /// assert_eq! (F::nan ().signum (), F::nan ());
    /// ```
    pub fn signum(&self) -> Self {
        match *self {
            GenericFraction::NaN => GenericFraction::NaN,

            GenericFraction::Infinity(Sign::Plus) => {
                GenericFraction::Rational(Sign::Plus, Ratio::new(T::one(), T::one()))
            }
            GenericFraction::Infinity(Sign::Minus) => {
                GenericFraction::Rational(Sign::Minus, Ratio::new(T::one(), T::one()))
            }

            GenericFraction::Rational(Sign::Plus, _) => {
                GenericFraction::Rational(Sign::Plus, Ratio::new(T::one(), T::one()))
            }
            GenericFraction::Rational(Sign::Minus, _) => {
                GenericFraction::Rational(Sign::Minus, Ratio::new(T::one(), T::one()))
            }
        }
    }

    /// Returns true if the sign is positive
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert! (F::new (1u8, 2u8).is_sign_positive ());
    /// assert! (F::infinity ().is_sign_positive ());
    /// assert! (! F::nan ().is_sign_positive ());
    /// ```
    pub const fn is_sign_positive(&self) -> bool {
        match *self {
            GenericFraction::NaN => false,
            GenericFraction::Infinity(Sign::Plus) => true,
            GenericFraction::Infinity(Sign::Minus) => false,
            GenericFraction::Rational(Sign::Plus, _) => true,
            GenericFraction::Rational(Sign::Minus, _) => false,
        }
    }

    /// Returns true if the sign is negative
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert! (F::new_neg (1u8, 2u8).is_sign_negative ());
    /// assert! (F::neg_zero ().is_sign_negative ());
    /// assert! (F::neg_infinity ().is_sign_negative ());
    /// assert! (! F::nan ().is_sign_negative ());
    /// ```
    pub const fn is_sign_negative(&self) -> bool {
        match *self {
            GenericFraction::NaN => false,
            GenericFraction::Infinity(Sign::Plus) => false,
            GenericFraction::Infinity(Sign::Minus) => true,
            GenericFraction::Rational(Sign::Plus, _) => false,
            GenericFraction::Rational(Sign::Minus, _) => true,
        }
    }

    /// self.clone () * a + b
    ///
    /// Added for interface compatibility with float types
    pub fn mul_add(&self, a: Self, b: Self) -> Self {
        self.clone() * a + b
    }

    /// Takes the reciprocal (inverse) of the value (1/x)
    ///
    /// # Examples
    ///
    /// ```
    /// use fraction::GenericFraction;
    /// type F = GenericFraction<u8>;
    ///
    /// assert_eq! (F::new (1u8, 2u8).recip (), F::new (2u8, 1u8));
    /// assert_eq! (F::new (0u8, 1u8).recip (), F::infinity ());
    /// assert_eq! (F::infinity ().recip (), F::new (0u8, 1u8));
    /// assert_eq! (F::nan ().recip (), F::nan ());
    /// ```
    pub fn recip(&self) -> Self {
        match *self {
            GenericFraction::NaN => GenericFraction::NaN,
            GenericFraction::Infinity(_) => {
                GenericFraction::Rational(Sign::Plus, Ratio::new(T::zero(), T::one()))
            }
            GenericFraction::Rational(s, ref r) if r.is_zero() => GenericFraction::Infinity(s),
            GenericFraction::Rational(s, ref r) => GenericFraction::Rational(s, r.recip()),
        }
    }

    /* ... Some stuff here that has not been implemented for Ratio<T> ... */
}

impl<T: Clone + GenericInteger> fmt::Display for GenericFraction<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let format = display::Format::new(formatter);
        display::format_fraction(self, formatter, &format)
    }
}

macro_rules! fraction_from_generic_int {
    ( $($F:ty),* ) => {
        $(
        impl<T> From<$F> for GenericFraction<T>
        where
            T: Clone + Integer + GenericInteger + CheckedAdd + CheckedMul + CheckedSub,
            $F: GenericInteger + CheckedAdd + CheckedDiv + CheckedMul + CheckedSub + PartialOrd
        {
            fn from(val: $F) -> GenericFraction<T> {
                if let Some((sign, value)) = read_generic_integer::<T, $F>(val) {
                    GenericFraction::Rational(sign, Ratio::new (value, T::one()))
                } else {
                    GenericFraction::nan()
                }
            }
        }
        )*
    };
}

#[cfg(feature = "with-bigint")]
fraction_from_generic_int!(BigUint, BigInt);

fraction_from_generic_int!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);

macro_rules! generic_fraction_from_float {
    ( $($from:ty),*) => {
        $(
        impl<T: Clone + FromPrimitive + Integer + CheckedAdd + CheckedMul + CheckedSub> From<$from> for GenericFraction<T> {
            fn from(val: $from) -> GenericFraction<T> {
                if val.is_nan () { return Self::NaN };
                if val.is_infinite () { return Self::Infinity (if val.is_sign_negative () { Sign::Minus } else { Sign::Plus }) };

                let sign = if val < 0.0 { Sign::Minus } else { Sign::Plus };

                // Using https://math.stackexchange.com/a/1049723/17452 , but rely on Ratio::new() to compute the gcd.
                // Find the max precision of this number
                // Note: the power computations happen in i32 until the end.
                let mut p: i32 = 0;
                let mut new_val = val;
                let ten: $from = 10.0;
                let fallback_to_string_conversion = || Self::from_str(&format!("{:+}", val)).unwrap_or(Self::NaN);
                loop {
                    if (new_val.floor() - new_val).abs() < <$from>::EPSILON {
                        // Yay, we've found the precision of this number
                        break;
                    }
                    // Multiply by the precision
                    // Note: we multiply by powers of ten to avoid this kind of round error with f32s:
                    // https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=b760579f103b7192c20413ebbe167b90
                    p += 1;
                    new_val = val * ten.powi(p);
                    if new_val.is_infinite() {
                        return fallback_to_string_conversion();
                    }
                }

                // We store the sign of the ratio externally, so let's oppose the numerator if need be.
                // The denominator is always positive.
                if new_val < 0.0 {
                    new_val = -new_val;
                }

                let numer: T = match T::from_f64(new_val.into()) {
                    Some(v) => v,
                    None => {
                        return fallback_to_string_conversion();
                    }
                };
                let denom: T = match T::from_f64(ten.powi(p).into()) {
                    Some(v) => v,
                    None => {
                        return fallback_to_string_conversion();
                    }
                };

                Self::Rational(sign, Ratio::new(numer, denom))
            }
        }
        )*
    };
}

generic_fraction_from_float!(f32, f64);

impl<T, N, D> From<(N, D)> for GenericFraction<T>
where
    T: Clone + GenericInteger,
    N: GenericInteger + PartialOrd,
    D: GenericInteger + PartialOrd,
{
    fn from(pair: (N, D)) -> GenericFraction<T> {
        GenericFraction::new_generic(Sign::Plus, pair.0, pair.1).unwrap_or(GenericFraction::NaN)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/generic_fraction/tests_1337.rs"));
