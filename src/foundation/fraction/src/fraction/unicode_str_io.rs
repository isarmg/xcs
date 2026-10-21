use crate::fraction::{generic_fraction::parse_magnitude, GenericFraction};
use crate::{error::ParseError, Sign};
use num::rational::Ratio;
use num::{CheckedAdd, CheckedMul, Zero};
use std::{fmt, str};
use Integer;

pub struct UnicodeDisplay<'a, T: Clone + Integer>(&'a GenericFraction<T>);
pub struct SupSubDisplay<'a, T: Clone + Integer>(&'a UnicodeDisplay<'a, T>);

fn checked_mixed_numerator<T>(trunc: &T, numer: &T, denom: &T) -> Result<T, ParseError>
where
    T: CheckedAdd + CheckedMul,
{
    let scaled_trunc = trunc.checked_mul(denom).ok_or(ParseError::ParseIntError)?;
    scaled_trunc
        .checked_add(numer)
        .ok_or(ParseError::ParseIntError)
}

impl<'a, T> fmt::Display for UnicodeDisplay<'a, T>
where
    T: Clone + Integer + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            GenericFraction::NaN => write!(f, "NaN"),
            GenericFraction::Infinity(s) => {
                write!(f, "{}∞", s)
            }
            GenericFraction::Rational(s, r) => {
                write!(f, "{}{}", s, r.numer())?;
                if !r.denom().is_one() {
                    write!(f, "⁄{}", r.denom())?;
                }
                Ok(())
            }
        }
    }
}

impl<'a, T: Clone + Integer + fmt::Display> UnicodeDisplay<'a, T> {
    /// Display the fraction using FRACTION SLASH '\u{2044}' '⁄' as a mixed fraction e.g. "1⁤1⁄2"
    /// Will put INVISIBLE PLUS '\u{2064}' as a separator '⁤'
    /// If you have font support, this is the way Unicode wants to display fractions
    /// ```
    /// use fraction::Fraction;
    /// assert_eq!(
    ///   format!("{}", Fraction::new(1u8,2u8).get_unicode_display().mixed()),
    ///   "1⁄2"
    /// );
    /// assert_eq!(
    ///   format!("{}", Fraction::new(3u8,2u8).get_unicode_display().mixed()),
    ///   "1⁤1⁄2"
    /// );
    /// ```
    pub fn mixed(&self) -> impl fmt::Display + '_ {
        struct D<'a, T: Clone + Integer + fmt::Display>(&'a UnicodeDisplay<'a, T>);
        impl<'a, T: Clone + Integer + fmt::Display> fmt::Display for D<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match &self.0 .0 {
                    GenericFraction::Rational(s, r) if r.fract() != *r && !r.denom().is_one() => {
                        write!(
                            f,
                            "{}{}\u{2064}{}⁄{}",
                            s,
                            r.trunc().numer(),
                            r.fract().numer(),
                            r.fract().denom()
                        )
                    }
                    _ => write!(f, "{}", self.0),
                }
            }
        }
        D(self)
    }

    /// Display the fraction using super/subscript
    /// This will look OK without font support
    /// ```
    /// use fraction::Fraction;
    /// assert_eq!(
    ///   format!("{}", Fraction::new(1u8,2u8).get_unicode_display().supsub()),
    ///   "¹/₂"
    /// );
    /// assert_eq!(
    ///   format!("{}", Fraction::new(3u8,2u8).get_unicode_display().supsub()),
    ///   "³/₂"
    /// );
    /// ```
    pub fn supsub(&'a self) -> SupSubDisplay<'a, T> {
        SupSubDisplay(self)
    }
}

impl<'a, T: Clone + Integer + fmt::Display> SupSubDisplay<'a, T> {
    /// Display the fraction as a mixed fraction using super/subscript e.g. "1¹/₂"
    /// This will look OK without font support
    /// ```
    /// use fraction::Fraction;
    /// assert_eq!(
    ///   format!("{}", Fraction::new(1u8,2u8).get_unicode_display().supsub().mixed()),
    ///   "¹/₂"
    /// );
    /// assert_eq!(
    ///   format!("{}", Fraction::new(3u8,2u8).get_unicode_display().supsub().mixed()),
    ///   "1¹/₂"
    /// );
    /// ```
    pub fn mixed(&self) -> impl fmt::Display + '_ {
        struct D<'a, T: Clone + Integer + fmt::Display>(&'a SupSubDisplay<'a, T>);
        impl<'a, T: Clone + Integer + fmt::Display> fmt::Display for D<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match &self.0 .0 .0 {
                    GenericFraction::Rational(s, r) if r.fract() != *r && !r.denom().is_one() => {
                        write!(
                            f,
                            "{}{}{}",
                            s,
                            r.trunc().numer(),
                            GenericFraction::Rational(Sign::Plus, r.fract().clone())
                                .get_unicode_display()
                                .supsub()
                        )
                    }
                    _ => write!(f, "{}", self.0),
                }
            }
        }
        D(self)
    }
}

impl<'a, T: Clone + Integer + fmt::Display> fmt::Display for SupSubDisplay<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 .0 {
            GenericFraction::Rational(s, r) => {
                write!(
                    f,
                    "{}{}/{}",
                    s,
                    r.numer()
                        .to_string()
                        .chars()
                        .map(|c| match c {
                            '1' => '¹',
                            '2' => '²',
                            '3' => '³',
                            '4' => '⁴',
                            '5' => '⁵',
                            '6' => '⁶',
                            '7' => '⁷',
                            '8' => '⁸',
                            '9' => '⁹',
                            '0' => '⁰',
                            _ => '?',
                        })
                        .collect::<String>(),
                    r.denom()
                        .to_string()
                        .chars()
                        .map(|c| match c {
                            '1' => '₁',
                            '2' => '₂',
                            '3' => '₃',
                            '4' => '₄',
                            '5' => '₅',
                            '6' => '₆',
                            '7' => '₇',
                            '8' => '₈',
                            '9' => '₉',
                            '0' => '₀',
                            c => c,
                        })
                        .collect::<String>(),
                )
            }
            _ => write!(f, "{}", self.0),
        }
    }
}

impl<T: Clone + Integer + fmt::Display> GenericFraction<T> {
    /// Display the fraction using FRACTION SLASH '\u{2044}' '⁄'
    /// If you have font support, this is the way Unicode wants to display fractions
    /// ```
    /// use fraction::Fraction;
    /// assert_eq!(
    ///   format!("{}", Fraction::new(1u8,2u8).get_unicode_display()),
    ///   "1⁄2"
    /// );
    /// assert_eq!(
    ///   format!("{}", Fraction::new(3u8,2u8).get_unicode_display()),
    ///   "3⁄2"
    /// );
    /// ```
    pub fn get_unicode_display(&self) -> UnicodeDisplay<'_, T> {
        UnicodeDisplay(self)
    }
}

impl<T: Clone + Integer + From<u8>> GenericFraction<T> {
    /// Parse a unicode string
    /// The string can be:
    /// - A normal fraction e.g. "1/2"
    /// - A vulgar fraction e.g. '½'
    /// - ~A mixed vulgar fraction "1½"~
    /// - A unicode fraction e.g. "1⁄2" where '⁄' can be any of:
    ///   - '/' ASCII SOLIDUS
    ///   - '⁄' FRACTION SLASH
    ///   - '∕' DIVISION SLASH
    ///   - '÷' DIVISION SIGN
    /// - A mixed unicode fraction: "1\u{2063}1⁄2": 1⁤1⁄2
    ///   - '\u{2064}' INVISIBLE PLUS
    ///   - '\u{2063}' INVISIBLE SEPARATOR
    ///   - NOT ~'\u{2062}' INVISIBLE TIMES~
    /// - A super-subscript fraction "¹/₂"
    /// - A mixed super-subscript fraction  "1¹/₂"
    ///
    /// Focus is on being lenient towards input rather than being fast.
    /// For Unicode numeric-component forms, an optional leading sign applies to
    /// the complete value; numeric components in ordinary, mixed, and
    /// super/subscript forms cannot carry their own sign.
    ///
    /// Mixed-number components are combined with checked arithmetic before the
    /// ratio is constructed. If the raw combined numerator does not fit in
    /// `T`, parsing returns [`ParseError::ParseIntError`]. A zero denominator
    /// still produces the corresponding infinity or NaN special value.
    ///
    /// ```
    /// use fraction::Fraction;
    /// let v = vec![
    ///  ("1/2", Fraction::new(1u8,2u8)),
    ///  ("-1/2", Fraction::new_neg(1u8,2u8)),
    ///  ("½", Fraction::new(1u8,2u8)),
    ///  // ("1½", Fraction::new(3u8,2u8)),       // mixed vulgar fractions
    ///  // ("-1½", Fraction::new_neg(3u8,2u8)),  // currently not supported
    ///  ("1⁄2", Fraction::new(1u8,2u8)),
    ///  ("-1⁄2", Fraction::new_neg(1u8,2u8)),
    ///  ("1⁤1⁄2", Fraction::new(3u8,2u8)),
    ///  ("-1⁤1⁄2", Fraction::new_neg(3u8,2u8)),
    ///  ("¹/₂", Fraction::new(1u8,2u8)),
    ///  ("-¹/₂", Fraction::new_neg(1u8,2u8)),
    ///  ("1¹/₂", Fraction::new(3u8,2u8)),
    ///  ("-1¹/₂", Fraction::new_neg(3u8,2u8)),
    /// ];
    /// for (f_str, f) in v {
    ///   assert_eq!(Fraction::from_unicode_str(f_str), Ok(f))
    /// }
    /// ```
    /// Slash and fraction-slash inputs, including mixed forms, keep the same
    /// zero-denominator semantics: `1⁄0` yields positive infinity, `-1⁄0`
    /// yields negative infinity, and `0⁄0` yields NaN.
    /// ```
    /// use fraction::Fraction;
    ///
    /// assert_eq!(Fraction::from_unicode_str("1⁄0"), Ok(Fraction::infinity()));
    /// assert_eq!(Fraction::from_unicode_str("-1⁄0"), Ok(Fraction::neg_infinity()));
    /// assert_eq!(Fraction::from_unicode_str("0⁄0"), Ok(Fraction::nan()));
    /// ```
    pub fn from_unicode_str(input: &str) -> Result<Self, ParseError>
    where
        T: CheckedAdd + CheckedMul,
    {
        let s: &str;
        let sign = if input.starts_with('-') {
            s = input.strip_prefix('-').unwrap();
            Sign::Minus
        } else if input.starts_with('+') {
            s = input.strip_prefix('+').unwrap();
            Sign::Plus
        } else {
            s = input;
            Sign::Plus
        };
        if s.to_lowercase().starts_with("nan") {
            Ok(GenericFraction::nan())
        } else if s.starts_with('∞') || s.starts_with("inf") || s.starts_with("infty") {
            Ok(GenericFraction::Infinity(sign))
        // vulgar fractions
        } else if s.starts_with('½') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 2.into()),
            ))
        } else if s.starts_with('¼') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 4.into()),
            ))
        } else if s.starts_with('¾') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(3.into(), 4.into()),
            ))
        } else if s.starts_with('⅐') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 7.into()),
            ))
        } else if s.starts_with('⅑') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 9.into()),
            ))
        } else if s.starts_with('⅒') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 10.into()),
            ))
        } else if s.starts_with('⅓') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 3.into()),
            ))
        } else if s.starts_with('⅔') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(2.into(), 3.into()),
            ))
        } else if s.starts_with('⅕') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(1.into(), 5.into()),
            ))
        } else if s.starts_with('⅖') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(2.into(), 5.into()),
            ))
        } else if s.starts_with('⅗') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(3.into(), 5.into()),
            ))
        } else if s.starts_with('⅘') {
            Ok(GenericFraction::Rational(
                sign,
                Ratio::new_raw(4.into(), 5.into()),
            ))
        } else if let Some((first, denom_str)) = s.split_once(&['/', '⁄', '∕', '÷'][..]) {
            // allow for mixed fractions of the shape 1²/₃
            if let Some(idx) =
                first.find(&['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹', '⁰'][..])
            {
                let trunc = if idx.is_zero() {
                    T::zero()
                } else {
                    let Ok(t) = parse_magnitude::<T>(&first[..idx], 10) else {
                        return Err(ParseError::ParseIntError);
                    };
                    t
                };

                let Ok(numer) = parse_magnitude::<T>(
                    &first[idx..]
                        .chars()
                        .map(|c| match c {
                            '¹' => '1',
                            '²' => '2',
                            '³' => '3',
                            '⁴' => '4',
                            '⁵' => '5',
                            '⁶' => '6',
                            '⁷' => '7',
                            '⁸' => '8',
                            '⁹' => '9',
                            '⁰' => '0',
                            _ => '?',
                        })
                        .collect::<String>(),
                    10,
                ) else {
                    return Err(ParseError::ParseIntError);
                };
                let Ok(denom) = parse_magnitude::<T>(
                    // let n =
                    &denom_str
                        .chars()
                        .map(|c| match c {
                            '₁' => '1',
                            '₂' => '2',
                            '₃' => '3',
                            '₄' => '4',
                            '₅' => '5',
                            '₆' => '6',
                            '₇' => '7',
                            '₈' => '8',
                            '₉' => '9',
                            '₀' => '0',
                            c => c,
                        })
                        .collect::<String>(),
                    10,
                ) else {
                    return Err(ParseError::ParseIntError);
                };
                let numer = checked_mixed_numerator(&trunc, &numer, &denom)?;
                Ok(Self::new_signed(sign, numer, denom))
            } else if let Some((trunc_str, numer_str)) =
                // also allow for mixed fractions to be parsed: `1⁤1⁄2`
                // allowed invisible separators: \u{2064} \u{2063}
                // '+' is disallowed, bc it would be confusing with -1+1/2
                first.split_once(&['\u{2064}', '\u{2063}'][..])
            {
                let Ok(numer) = parse_magnitude::<T>(numer_str, 10) else {
                    return Err(ParseError::ParseIntError);
                };
                let Ok(trunc) = parse_magnitude::<T>(trunc_str, 10) else {
                    return Err(ParseError::ParseIntError);
                };
                let Ok(denom) = parse_magnitude::<T>(denom_str, 10) else {
                    return Err(ParseError::ParseIntError);
                };
                let numer = checked_mixed_numerator(&trunc, &numer, &denom)?;
                Ok(Self::new_signed(sign, numer, denom))
            } else {
                let Ok(numer) = parse_magnitude::<T>(first, 10) else {
                    return Err(ParseError::ParseIntError);
                };

                let Ok(denom) = parse_magnitude::<T>(denom_str, 10) else {
                    return Err(ParseError::ParseIntError);
                };

                Ok(Self::new_signed(sign, numer, denom))
            }
        } else {
            let Ok(val) = parse_magnitude::<T>(s, 10) else {
                return Err(ParseError::ParseIntError);
            };
            Ok(GenericFraction::Rational(sign, Ratio::new(val, T::one())))
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/unicode_str_io/tests_438.rs"));
