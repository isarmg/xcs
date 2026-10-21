#[cfg(test)]
mod tests {
    #[cfg(feature = "with-bigint")]
    use prelude::{BigFraction, BigUint};

    use super::super::super::{One, Zero};
    use prelude::GenericFraction;

    type F32 = GenericFraction<i32>;
    type F64 = GenericFraction<u64>;

    #[test]
    fn display() {
        assert_eq!("NaN", format!("{}", F32::nan()));
        assert_eq!(format!("{}", ::std::f64::NAN), format!("{}", F32::nan()));

        assert_eq!("NaN", format!("{:+}", F32::nan()));
        assert_eq!(
            format!("{:+}", ::std::f64::NAN),
            format!("{:+}", F32::nan())
        );
        assert_eq!("==NaN==", format!("{:=^7}", F32::nan()));
        assert_eq!("=NaN==", format!("{:=^6}", F32::nan()));
        assert_eq!("NaN====", format!("{:=<7}", F32::nan()));
        assert_eq!("====NaN", format!("{:=>7}", F32::nan()));
        assert_eq!("0000NaN", format!("{:=>07}", F32::nan()));
        assert_eq!("0000NaN", format!("{:07}", F32::nan()));
        assert_eq!("0000NaN", format!("{:+07}", F32::nan()));

        assert_eq!("inf", format!("{}", F32::infinity()));
        assert_eq!(
            format!("{}", ::std::f64::INFINITY),
            format!("{}", F32::infinity())
        );
        assert_eq!("==inf==", format!("{:=^7}", F32::infinity()));
        assert_eq!("inf====", format!("{:=<7}", F32::infinity()));
        assert_eq!("====inf", format!("{:=>7}", F32::infinity()));
        assert_eq!("0000inf", format!("{:=<07}", F32::infinity()));
        assert_eq!("+000inf", format!("{:=<+07}", F32::infinity()));

        assert_eq!("+inf", format!("{:+}", F32::infinity()));
        assert_eq!(
            format!("{:+}", ::std::f64::INFINITY),
            format!("{:+}", F32::infinity())
        );
        assert_eq!("=+inf==", format!("{:=^+7}", F32::infinity()));

        assert_eq!("-inf", format!("{}", F32::neg_infinity()));
        assert_eq!(
            format!("{}", ::std::f64::NEG_INFINITY),
            format!("{}", F32::neg_infinity())
        );
        assert_eq!("=-inf==", format!("{:=^7}", F32::neg_infinity()));

        assert_eq!("-inf", format!("{:+}", F32::neg_infinity()));
        assert_eq!(
            format!("{:+}", ::std::f64::NEG_INFINITY),
            format!("{:+}", F32::neg_infinity())
        );
        assert_eq!("=-inf==", format!("{:=^+7}", F32::neg_infinity()));

        // zero
        assert_eq!("0", format!("{}", F32::zero()));
        assert_eq!(format!("{}", 0.0f64), format!("{}", F32::zero()));

        assert_eq!("+0", format!("{:+}", F32::zero()));
        assert_eq!(format!("{:+}", 0.0f64), format!("{:+}", F32::zero()));

        assert_eq!("0", format!("{:.2}", F32::zero()));
        assert_eq!("+0", format!("{:+.2}", F32::zero()));
        assert_eq!("0", format!("{:-.2}", F32::zero()));

        assert_eq!("0.00", format!("{:#.2}", F32::zero()));
        assert_eq!("+0.00", format!("{:+#.2}", F32::zero()));
        assert_eq!("0.00", format!("{:-#.2}", F32::zero()));

        // neg zero
        assert_eq!("-0", format!("{}", F32::neg_zero()));
        assert_eq!("-0", format!("{:+}", F32::neg_zero()));
        assert_eq!("0", format!("{:-}", F32::neg_zero()));

        assert_eq!("-0", format!("{:.2}", F32::neg_zero()));
        assert_eq!("-0", format!("{:+.2}", F32::neg_zero()));
        assert_eq!("0", format!("{:-.2}", F32::neg_zero()));

        // positive whole
        assert_eq!("1", format!("{}", F32::one()));
        assert_eq!("+1", format!("{:+}", F32::one()));
        assert_eq!("1", format!("{:-}", F32::one()));

        assert_eq!("0001", format!("{:04}", F32::one()));
        assert_eq!("+001", format!("{:+04}", F32::one()));

        assert_eq!("=1==", format!("{:=^4}", F32::one()));
        assert_eq!("===1", format!("{:=>4}", F32::one()));
        assert_eq!("1===", format!("{:=<4}", F32::one()));
        assert_eq!("=+1=", format!("{:=^+4}", F32::one()));

        // negative whole
        assert_eq!("-1", format!("{}", -F32::one()));
        assert_eq!("-1", format!("{:+}", -F32::one()));
        assert_eq!("1", format!("{:-}", -F32::one()));

        assert_eq!("=-1=", format!("{:=^4}", -F32::one()));
        assert_eq!("=1==", format!("{:=^-4}", -F32::one()));
        assert_eq!("=-1=", format!("{:=^+4}", -F32::one()));

        // positive fraction
        assert_eq!("00.5", format!("{:04.1}", F32::new(1, 2)));
        assert_eq!("+0.5", format!("{:+04.1}", F32::new(1, 2)));
        assert_eq!("00.5", format!("{:-04.1}", F32::new(1, 2)));

        assert_eq!("0.5", format!("{:.16}", F32::new(1, 2)));
        assert_eq!("+0.5", format!("{:+.16}", F32::new(1, 2)));
        assert_eq!("+1.75", format!("{:=^+.4}", F32::new(7, 4)));
        assert_eq!("=+1.75==", format!("{:=^+8.4}", F32::new(7, 4)));
        assert_eq!("   +1.75", format!("{:+8.4}", F32::new(7, 4)));
        assert_eq!("   +1.75", format!("{:>+8.4}", F32::new(7, 4)));
        assert_eq!("+1.75   ", format!("{:<+8.4}", F32::new(7, 4)));
        assert_eq!("+0001.75", format!("{:<+08.4}", F32::new(7, 4)));

        // negative fraction
        assert_eq!("-00.5", format!("{:05.2}", -F32::new(1, 2)));
        assert_eq!("-00.5", format!("{:05.2}", -F32::new(1, 2)));
        assert_eq!("-00.5", format!("{:+05.2}", -F32::new(1, 2)));
        assert_eq!("000.5", format!("{:-05.2}", -F32::new(1, 2)));

        assert_eq!("-0.5", format!("{:.16}", -F32::new(1, 2)));
        assert_eq!("0.5", format!("{:-.16}", -F32::new(1, 2)));
        assert_eq!("-0.5", format!("{:+.16}", -F32::new(1, 2)));

        assert_eq!("-1.75", format!("{:=^+.4}", -F32::new(7, 4)));
        assert_eq!("=-1.75==", format!("{:=^+8.4}", -F32::new(7, 4)));
        assert_eq!("   -1.75", format!("{:+8.4}", -F32::new(7, 4)));
        assert_eq!("   -1.75", format!("{:>+8.4}", -F32::new(7, 4)));
        assert_eq!("-1.75   ", format!("{:<+8.4}", -F32::new(7, 4)));
        assert_eq!("-0001.75", format!("{:<+08.4}", -F32::new(7, 4)));

        assert_eq!("-1.75", format!("{:=^.4}", -F32::new(7, 4)));
        assert_eq!("=-1.75==", format!("{:=^8.4}", -F32::new(7, 4)));
        assert_eq!("   -1.75", format!("{:8.4}", -F32::new(7, 4)));
        assert_eq!("   -1.75", format!("{:>8.4}", -F32::new(7, 4)));
        assert_eq!("-1.75   ", format!("{:<8.4}", -F32::new(7, 4)));
        assert_eq!("-0001.75", format!("{:<08.4}", -F32::new(7, 4)));

        assert_eq!("-01.7500", format!("{:<#08.4}", -F32::new(7, 4)));

        assert_eq!("1.75", format!("{:=^-.4}", -F32::new(7, 4)));
        assert_eq!("==1.75==", format!("{:=^-8.4}", -F32::new(7, 4)));
        assert_eq!("    1.75", format!("{:-8.4}", -F32::new(7, 4)));
        assert_eq!("    1.75", format!("{:>-8.4}", -F32::new(7, 4)));
        assert_eq!("1.75    ", format!("{:<-8.4}", -F32::new(7, 4)));
        assert_eq!("00001.75", format!("{:<-08.4}", -F32::new(7, 4)));

        // ratios
        assert_eq!("7/4", format!("{}", F32::new(7, 4)));
        assert_eq!("7/4", format!("{:-}", F32::new(7, 4)));
        assert_eq!("+7/4", format!("{:+}", F32::new(7, 4)));

        assert_eq!("-7/4", format!("{}", -F32::new(7, 4)));
        assert_eq!("7/4", format!("{:-}", -F32::new(7, 4)));
        assert_eq!("-7/4", format!("{:+}", -F32::new(7, 4)));

        assert_eq!("07/4", format!("{:04}", F32::new(7, 4)));
        assert_eq!("-7/4", format!("{:+04}", -F32::new(7, 4)));

        assert_eq!("==7/4==", format!("{:=^7}", F32::new(7, 4)));
        assert_eq!("=-7/4==", format!("{:=^7}", -F32::new(7, 4)));
        assert_eq!("    7/4", format!("{:7}", F32::new(7, 4)));
        assert_eq!("7/4    ", format!("{:<7}", F32::new(7, 4)));
        assert_eq!("-7/4   ", format!("{:<7}", -F32::new(7, 4)));
        assert_eq!("+7/4   ", format!("{:<+7}", F32::new(7, 4)));
        assert_eq!("+0007/4", format!("{:<+07}", F32::new(7, 4)));

        // former format_as_decimal tests
        assert_eq!("NaN", format!("{:.64}", F32::from(::std::f32::NAN)));
        assert_eq!("inf", format!("{:.64}", F32::from(::std::f32::INFINITY)));
        assert_eq!(
            "-inf",
            format!("{:.64}", F32::from(::std::f32::NEG_INFINITY))
        );

        assert_eq!("0.75", format!("{:.64}", F32::from(0.75)));
        assert_eq!("-0.75", format!("{:.64}", F32::from(-0.75)));
        assert_eq!("0.33", format!("{:.64}", F32::from((33, 100))));
        assert_eq!(
            "0.0000000456",
            format!("{:.64}", F64::new(456u64, 10000000000u64))
        );

        #[cfg(feature = "with-bigint")]
        {
            let fra = BigFraction::new(
                BigUint::from(42u8),
                BigUint::from(1000000000000000u64)
                    * BigUint::from(1000000000000000u64)
                    * BigUint::from(1000000000000000u64),
            );

            assert_eq!(
                "0.000000000000000000000000000000000000000000042",
                format!("{:.64}", fra)
            );
        }
    }
}
