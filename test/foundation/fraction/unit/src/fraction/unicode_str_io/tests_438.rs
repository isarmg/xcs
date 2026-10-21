#[cfg(test)]
mod tests {

    use crate::{error::ParseError, Fraction, GenericFraction, Sign};
    use num::{One, Zero};
    #[cfg(feature = "with-bigint")]
    use std::str::FromStr;

    #[test]
    fn from_unicode_str_mixed_rejects_reported_u64_overflow() {
        let inputs = ["18446744073709551615¹/₂", "18446744073709551615\u{2064}1⁄2"];

        for input in inputs {
            assert_eq!(
                GenericFraction::<u64>::from_unicode_str(input),
                Err(ParseError::ParseIntError)
            );
        }
    }

    #[test]
    fn from_unicode_str_mixed_rejects_whole_part_multiplication_overflow() {
        for input in ["128¹/₂", "128\u{2064}1/2"] {
            assert_eq!(
                GenericFraction::<u8>::from_unicode_str(input),
                Err(ParseError::ParseIntError)
            );
        }
    }

    #[test]
    fn from_unicode_str_mixed_rejects_fractional_addition_overflow() {
        for input in ["255¹/₁", "255\u{2064}1/1"] {
            assert_eq!(
                GenericFraction::<u8>::from_unicode_str(input),
                Err(ParseError::ParseIntError)
            );
        }
    }

    #[test]
    fn from_unicode_str_mixed_accepts_maximum_fitting_raw_numerator_with_outer_signs() {
        assert_eq!(
            GenericFraction::<u8>::from_unicode_str("127¹/₂"),
            Ok(GenericFraction::<u8>::new(255, 2))
        );
        assert_eq!(
            GenericFraction::<u8>::from_unicode_str("+127\u{2064}1/2"),
            Ok(GenericFraction::<u8>::new(255, 2))
        );
        assert_eq!(
            GenericFraction::<u8>::from_unicode_str("-127¹/₂"),
            Ok(GenericFraction::<u8>::new_neg(255, 2))
        );
    }

    #[test]
    fn from_unicode_str_mixed_rejects_raw_overflow_before_reduction() {
        for input in ["127²/₂", "127\u{2064}2/2"] {
            assert_eq!(
                GenericFraction::<u8>::from_unicode_str(input),
                Err(ParseError::ParseIntError)
            );
        }
    }

    #[test]
    fn from_unicode_str_mixed_preserves_zero_denominator_semantics() {
        assert_eq!(
            Fraction::from_unicode_str("1\u{2064}1/0"),
            Ok(Fraction::infinity())
        );
        assert_eq!(Fraction::from_unicode_str("1¹/₀"), Ok(Fraction::infinity()));
        assert_eq!(
            Fraction::from_unicode_str("0\u{2064}0/0"),
            Ok(Fraction::nan())
        );
    }

    #[test]
    fn from_unicode_str_unsigned_rejects_component_sign() {
        assert_eq!(
            Fraction::from_unicode_str("1⁄+2"),
            Err(ParseError::ParseIntError)
        );
    }

    #[test]
    fn from_unicode_str_component_sign_rejected_for_signed_storage() {
        type SignedFrac = GenericFraction<i16>;

        for (input, sign, numer, denom) in [
            ("1⁄2", Sign::Plus, 1i16, 2i16),
            ("+1⁄2", Sign::Plus, 1i16, 2i16),
            ("-1⁄2", Sign::Minus, 1i16, 2i16),
            ("-1⁤1⁄2", Sign::Minus, 3i16, 2i16),
            ("-1¹/₂", Sign::Minus, 3i16, 2i16),
        ] {
            match SignedFrac::from_unicode_str(input).unwrap() {
                SignedFrac::Rational(actual_sign, ratio) => {
                    assert_eq!(sign, actual_sign);
                    assert_eq!(&numer, ratio.numer());
                    assert_eq!(&denom, ratio.denom());
                    assert!(*ratio.numer() >= 0);
                    assert!(*ratio.denom() >= 0);
                }
                other => panic!("expected rational, got {:?}", other),
            }
        }

        for input in [
            "1/-2",
            "1/+2",
            "--1⁄2",
            "+-1⁄2",
            "1⁤-1⁄2",
            "1⁤+1⁄2",
            "-32768",
            "-32768⁄1",
            "1⁄-32768",
            "-1⁤1⁄-32768",
            "1¹/-32768",
        ] {
            assert_eq!(
                Err(ParseError::ParseIntError),
                SignedFrac::from_unicode_str(input),
                "input should not place a sign inside a Unicode component: {input}"
            );
        }
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn from_unicode_str_mixed_preserves_big_fraction_capacity() {
        use crate::BigFraction;

        let expected = BigFraction::from_str("36893488147419103231/2").unwrap();
        for input in ["18446744073709551615¹/₂", "18446744073709551615\u{2064}1⁄2"] {
            assert_eq!(BigFraction::from_unicode_str(input), Ok(expected.clone()));
        }
    }

    #[cfg(all(feature = "with-bigint", feature = "with-dynaint"))]
    #[test]
    fn from_unicode_str_mixed_preserves_dyna_fraction_capacity() {
        use crate::DynaFraction;

        let expected = DynaFraction::<u64>::from_str("36893488147419103231/2").unwrap();
        for input in ["18446744073709551615¹/₂", "18446744073709551615\u{2064}1⁄2"] {
            assert_eq!(
                DynaFraction::<u64>::from_unicode_str(input),
                Ok(expected.clone())
            );
        }
    }

    #[test]
    fn test_fromto_str() {
        let test_vec = vec![
            ("NaN", Fraction::nan()),
            ("∞", Fraction::infinity()),
            ("-∞", Fraction::neg_infinity()),
            ("0", Fraction::zero()),
            ("1", Fraction::one()),
            ("-1", -Fraction::one()),
            ("5", Fraction::from(5)),
            ("1⁄2", Fraction::new(1u8, 2u8)),
            ("-1⁄2", Fraction::new_neg(1u8, 2u8)),
            ("3⁄2", Fraction::new(3u8, 2u8)),
            ("-3⁄2", Fraction::new_neg(3u8, 2u8)),
            ("12⁄23", Fraction::new(12u8, 23u8)),
        ];
        for (string, frac) in test_vec {
            println!("{} ?= {}", string, frac);
            assert_eq!(Fraction::from_unicode_str(string), Ok(frac));
            println!("{} ?= {}", string, frac);
            assert_eq!(format!("{}", frac.get_unicode_display()), string);
        }
    }

    #[test]
    fn test_alternate_parse() {
        let test_vec = vec![
            ("nan", Fraction::nan()),
            ("+∞", Fraction::infinity()),
            ("+1", Fraction::one()),
            ("+5", Fraction::from(5)),
            // vulgar fractions
            ("½", Fraction::new(1u8, 2u8)),
            ("-½", Fraction::new_neg(1u8, 2u8)),
            ("+½", Fraction::new(1u8, 2u8)),
            ("¼", Fraction::new(1u8, 4u8)),
            ("-¼", Fraction::new_neg(1u8, 4u8)),
            ("+¼", Fraction::new(1u8, 4u8)),
            ("¾", Fraction::new(3u8, 4u8)),
            ("-¾", Fraction::new_neg(3u8, 4u8)),
            ("+¾", Fraction::new(3u8, 4u8)),
            ("⅐", Fraction::new(1u8, 7u8)),
            ("-⅐", Fraction::new_neg(1u8, 7u8)),
            ("+⅐", Fraction::new(1u8, 7u8)),
            ("⅑", Fraction::new(1u8, 9u8)),
            ("-⅑", Fraction::new_neg(1u8, 9u8)),
            ("+⅑", Fraction::new(1u8, 9u8)),
            ("⅒", Fraction::new(1u8, 10u8)),
            ("-⅒", Fraction::new_neg(1u8, 10u8)),
            ("+⅒", Fraction::new(1u8, 10u8)),
            ("⅓", Fraction::new(1u8, 3u8)),
            ("-⅓", Fraction::new_neg(1u8, 3u8)),
            ("+⅓", Fraction::new(1u8, 3u8)),
            ("⅔", Fraction::new(2u8, 3u8)),
            ("-⅔", Fraction::new_neg(2u8, 3u8)),
            ("+⅔", Fraction::new(2u8, 3u8)),
            ("⅕", Fraction::new(1u8, 5u8)),
            ("-⅕", Fraction::new_neg(1u8, 5u8)),
            ("+⅕", Fraction::new(1u8, 5u8)),
            ("⅖", Fraction::new(2u8, 5u8)),
            ("-⅖", Fraction::new_neg(2u8, 5u8)),
            ("+⅖", Fraction::new(2u8, 5u8)),
            ("⅗", Fraction::new(3u8, 5u8)),
            ("-⅗", Fraction::new_neg(3u8, 5u8)),
            ("+⅗", Fraction::new(3u8, 5u8)),
            ("⅘", Fraction::new(4u8, 5u8)),
            ("-⅘", Fraction::new_neg(4u8, 5u8)),
            ("+⅘", Fraction::new(4u8, 5u8)),
            // supsub with other delim
            ("¹⁄₃", Fraction::new(1u8, 3u8)),
            ("1¹⁄₃", Fraction::new(4u8, 3u8)),
        ];
        for (string, frac) in test_vec {
            println!("{} ?= {}", string, frac);
            assert_eq!(Fraction::from_unicode_str(string), Ok(frac));
        }
    }

    #[test]
    fn test_fromto_supsub() {
        let test_vec = vec![
            // super/subscript
            ("¹²/₂₃", Fraction::new(12u8, 23u8)),
            ("¹²³⁴⁵⁶⁷⁸⁹⁰/₂₃", Fraction::new(1234567890u64, 23u8)),
            ("²³/₁₂₃₄₅₆₇₈₉₀", Fraction::new(23u8, 1234567890u64)),
        ];
        for (string, frac) in test_vec {
            println!("{} ?= {}", string, frac);
            assert_eq!(Fraction::from_unicode_str(string), Ok(frac));
            println!("{} ?= {}", string, frac);
            assert_eq!(format!("{}", frac.get_unicode_display().supsub()), string);
        }
    }

    #[test]
    fn test_fromto_supsub_mixed() {
        let test_vec = vec![
            // super/subscript mixed
            ("1¹/₂", Fraction::new(3u8, 2u8)),
            ("¹/₂", Fraction::new(1u8, 2u8)),
        ];
        for (string, frac) in test_vec {
            println!("{} ?= {}", string, frac);
            assert_eq!(Fraction::from_unicode_str(string), Ok(frac));
            println!("{} ?= {}", string, frac);
            assert_eq!(
                format!("{}", frac.get_unicode_display().supsub().mixed()),
                string
            );
        }
    }

    #[test]
    fn test_fromto_mixed() {
        let test_vec = vec![
            ("NaN", Fraction::nan()),
            ("∞", Fraction::infinity()),
            ("-∞", Fraction::neg_infinity()),
            ("0", Fraction::zero()),
            ("1", Fraction::one()),
            ("-1", -Fraction::one()),
            ("5", Fraction::from(5)),
            ("1\u{2064}1⁄2", Fraction::new(3u8, 2u8)),
            ("-1\u{2064}1⁄2", Fraction::new_neg(3u8, 2u8)),
            ("1⁄2", Fraction::new(1u8, 2u8)),
            ("-1⁄2", Fraction::new_neg(1u8, 2u8)),
        ];
        for (string, frac) in test_vec {
            println!("{} ?= {}", string, frac);
            let f_test = Fraction::from_unicode_str(string);
            assert_eq!(f_test, Ok(frac));
            assert_eq!(format!("{}", frac.get_unicode_display().mixed()), string);
        }
    }

    #[test]
    fn from_unicode_str_zero_denominator() {
        let test_vec = vec![
            ("1\u{2064}1/0", Fraction::infinity()),
            ("1/0", Fraction::infinity()),
            ("0/0", Fraction::nan()),
            ("²/₀", Fraction::infinity()),
            ("⁰/₀", Fraction::nan()),
            ("1⁄0", Fraction::infinity()),
            ("5⁄0", Fraction::infinity()),
            ("-1⁄0", Fraction::neg_infinity()),
            ("1\u{2064}0/0", Fraction::nan()),
            ("1⁰/₀", Fraction::nan()),
        ];
        for (string, fraction) in test_vec {
            assert_eq!(Fraction::from_unicode_str(string), Ok(fraction));
        }
    }

    #[test]
    fn test_from_fail() {
        // TODO: "nanBOGUS" and "∞BOGUS" will parse.
        // Either make that everything with BOGUS
        // after will parse, or make ^those fail.
        let test_vec = vec![
            "asdf",
            "+1BOGUS",
            "+5BOGUS",
            "1⁤1⁄2BOGUS",
            "1⁣1⁄2BOGUS",
            "-1⁤1⁄2BOGUS",
            "1⁢1⁄2", // uses INVISIBLE_TIMES
        ];
        for s in test_vec {
            println!("{}", s);
            assert_eq!(
                Fraction::from_unicode_str(s),
                Err(ParseError::ParseIntError)
            )
        }
    }

    #[test]
    fn test_fromstr_fraction_ops() {
        let test_vec = vec!["1", "1/2", "3/2"];
        for s in test_vec {
            let f = Fraction::from_unicode_str(s).unwrap();
            assert_eq!(f * Fraction::one(), f);
            assert_eq!(f + Fraction::zero(), f);
        }
    }
}
