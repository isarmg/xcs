#[cfg(test)]
mod tests {
    use crate::{approx::Accuracy, BigFraction, GenericFraction};
    use ::numeric::collection::{traits::Pow, BigUint, Zero};
    use std::str::FromStr;

    /// Converts `f` to a string. This is useful because `f.to_string()` will round sometimes.
    /// Ideal tests would use this, but it takes far too long to convert `BigUint`s to strings for
    /// that to be realistic.
    #[allow(dead_code)]
    fn fraction_to_decimal_string(f: &BigFraction, decimal_places: usize) -> String {
        let GenericFraction::Rational(sign, ratio) = f else {
            return f.to_string();
        };

        let sign_str = match sign {
            crate::Sign::Plus => "",
            crate::Sign::Minus => "-",
        };

        let integer = ratio.to_integer();

        let multiplier: BigUint = Pow::pow(&BigUint::from(10_u8), decimal_places);
        let fractional = (ratio.fract() * multiplier).to_integer();

        // Since `fractional` is an integer, converting it to a string removes any leading zeros.
        // We're using it to represent the fractional part of the number, though, so we need to add
        // back those leading zeros.
        format!("{sign_str}{integer}.{fractional:0>decimal_places$}")
    }

    fn test_sqrt_of(value: impl std::borrow::Borrow<BigFraction>, simplify: bool, n: usize) {
        let value = value.borrow();

        let accuracy = Accuracy::decimal_places(n);

        println!("calculating...");
        let sqrt = value.sqrt_with_accuracy_raw(&accuracy);

        let sqrt: BigFraction = if simplify {
            {
                println!("simplifying...");
                sqrt.simplified()
            }
        } else {
            sqrt
        }
        .into();

        println!("checking...");
        assert_eq!(
            accuracy.chopped_numerator_raw(
                &(sqrt.numer().unwrap() * sqrt.numer().unwrap()),
                &(sqrt.denom().unwrap() * sqrt.denom().unwrap()),
            ),
            accuracy.chopped_numerator_raw(value.numer().unwrap(), value.denom().unwrap())
        );
    }

    #[test]
    fn test_raw() {
        let values: [BigFraction; 5] = [
            BigFraction::from(2),
            BigFraction::from(123_654),
            BigFraction::from(123_655),
            BigFraction::from_str(
                "5735874745115151552958367280658028638020529468164964850251033802750\
            727314244020586751748892724760644\
            /\
            478953213143537128483961697945367179924659061093095449962100933428918126\
            6216833845985099376094324166",
            )
            .unwrap(),
            BigFraction::from(0.2),
        ];

        const ACCURACIES: [usize; 5] = [10, 100, 1000, 10000, 100_000];

        for value in &values {
            for accuracy in ACCURACIES {
                println!("sqrt({value}) to {accuracy} d.p., unsimplified");
                test_sqrt_of(value, false, accuracy);
            }
        }
    }

    #[test]
    fn test_negative() {
        let x: GenericFraction<u8> = (-2).into();
        assert_eq!(x.sqrt(10), GenericFraction::NaN);
    }

    #[test]
    fn test_abs() {
        let x: GenericFraction<u8> = 2.into();
        assert_eq!(x.sqrt(10), (-x).sqrt_abs(10));
    }

    #[test]
    fn test_weird_numbers() {
        let x: GenericFraction<u8> = f32::NAN.into();
        assert_eq!(GenericFraction::NaN, x.sqrt(10));

        let x: GenericFraction<u8> = f32::INFINITY.into();
        assert_eq!(GenericFraction::Infinity(crate::Sign::Plus), x.sqrt(10));

        let x: GenericFraction<u8> = f32::NEG_INFINITY.into();
        assert_eq!(GenericFraction::Infinity(crate::Sign::Plus), x.sqrt_abs(10));

        let x: GenericFraction<u8> = 0.into();
        assert!(x.sqrt(10).is_zero());
    }

    #[test]
    fn test_negative_inf() {
        let x: GenericFraction<u8> = f32::NEG_INFINITY.into();
        assert_eq!(x.sqrt(10), GenericFraction::NaN);
    }
}
