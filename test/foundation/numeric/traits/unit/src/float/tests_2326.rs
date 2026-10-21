#[cfg(test)]
mod tests {
    use core::f64::consts;

    const DEG_RAD_PAIRS: [(f64, f64); 7] = [
        (0.0, 0.),
        (22.5, consts::FRAC_PI_8),
        (30.0, consts::FRAC_PI_6),
        (45.0, consts::FRAC_PI_4),
        (60.0, consts::FRAC_PI_3),
        (90.0, consts::FRAC_PI_2),
        (180.0, consts::PI),
    ];

    #[test]
    fn convert_deg_rad() {
        use crate::traits::float::FloatCore;

        for &(deg, rad) in &DEG_RAD_PAIRS {
            assert!((FloatCore::to_degrees(rad) - deg).abs() < 1e-6);
            assert!((FloatCore::to_radians(deg) - rad).abs() < 1e-6);

            let (deg, rad) = (deg as f32, rad as f32);
            assert!((FloatCore::to_degrees(rad) - deg).abs() < 1e-5);
            assert!((FloatCore::to_radians(deg) - rad).abs() < 1e-5);
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn convert_deg_rad_std() {
        for &(deg, rad) in &DEG_RAD_PAIRS {
            use crate::traits::Float;

            assert!((Float::to_degrees(rad) - deg).abs() < 1e-6);
            assert!((Float::to_radians(deg) - rad).abs() < 1e-6);

            let (deg, rad) = (deg as f32, rad as f32);
            assert!((Float::to_degrees(rad) - deg).abs() < 1e-5);
            assert!((Float::to_radians(deg) - rad).abs() < 1e-5);
        }
    }

    #[test]
    fn to_degrees_rounding() {
        use crate::traits::float::FloatCore;

        assert_eq!(
            FloatCore::to_degrees(1_f32),
            57.2957795130823208767981548141051703
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn extra_logs() {
        use crate::traits::float::{Float, FloatConst};

        fn check<F: Float + FloatConst>(diff: F) {
            let _2 = F::from(2.0).unwrap();
            assert!((F::LOG10_2() - F::log10(_2)).abs() < diff);
            assert!((F::LOG10_2() - F::LN_2() / F::LN_10()).abs() < diff);

            let _10 = F::from(10.0).unwrap();
            assert!((F::LOG2_10() - F::log2(_10)).abs() < diff);
            assert!((F::LOG2_10() - F::LN_10() / F::LN_2()).abs() < diff);
        }

        check::<f32>(1e-6);
        check::<f64>(1e-12);
    }

    #[test]
    #[cfg(feature = "std")]
    fn copysign() {
        use crate::traits::float::Float;
        test_copysign_generic(2.0_f32, -2.0_f32, f32::nan());
        test_copysign_generic(2.0_f64, -2.0_f64, f64::nan());
        test_copysignf(2.0_f32, -2.0_f32, f32::nan());
    }

    #[cfg(feature = "std")]
    fn test_copysignf(p: f32, n: f32, nan: f32) {
        use crate::traits::float::Float;
        use core::ops::Neg;

        assert!(p.is_sign_positive());
        assert!(n.is_sign_negative());
        assert!(nan.is_nan());

        assert_eq!(p, Float::copysign(p, p));
        assert_eq!(p.neg(), Float::copysign(p, n));

        assert_eq!(n, Float::copysign(n, n));
        assert_eq!(n.neg(), Float::copysign(n, p));

        assert!(Float::copysign(nan, p).is_sign_positive());
        assert!(Float::copysign(nan, n).is_sign_negative());
    }

    #[cfg(feature = "std")]
    fn test_copysign_generic<F: crate::traits::float::Float + ::core::fmt::Debug>(p: F, n: F, nan: F) {
        assert!(p.is_sign_positive());
        assert!(n.is_sign_negative());
        assert!(nan.is_nan());
        assert!(!nan.is_subnormal());

        assert_eq!(p, p.copysign(p));
        assert_eq!(p.neg(), p.copysign(n));

        assert_eq!(n, n.copysign(n));
        assert_eq!(n.neg(), n.copysign(p));

        assert!(nan.copysign(p).is_sign_positive());
        assert!(nan.copysign(n).is_sign_negative());
    }

    #[cfg(feature = "std")]
    fn test_subnormal<F: crate::traits::float::Float + ::core::fmt::Debug>() {
        let min_positive = F::min_positive_value();
        let lower_than_min = min_positive / F::from(2.0f32).unwrap();
        assert!(!min_positive.is_subnormal());
        assert!(lower_than_min.is_subnormal());
    }

    #[test]
    #[cfg(feature = "std")]
    fn subnormal() {
        test_subnormal::<f64>();
        test_subnormal::<f32>();
    }

    #[test]
    fn total_cmp() {
        use crate::traits::float::TotalOrder;
        use core::cmp::Ordering;


        fn check_eq<T: TotalOrder>(x: T, y: T) {
            assert_eq!(x.total_cmp(&y), Ordering::Equal);
        }
        fn check_lt<T: TotalOrder>(x: T, y: T) {
            assert_eq!(x.total_cmp(&y), Ordering::Less);
        }
        fn check_gt<T: TotalOrder>(x: T, y: T) {
            assert_eq!(x.total_cmp(&y), Ordering::Greater);
        }

        check_eq(f64::NAN, f64::NAN);
        check_eq(f32::NAN, f32::NAN);

        check_lt(-0.0_f64, 0.0_f64);
        check_lt(-0.0_f32, 0.0_f32);

        // x87 registers don't preserve the exact value of signaling NaN:
        // https://github.com/rust-lang/rust/issues/115567
        #[cfg(not(target_arch = "x86"))]
        {
            let s_nan = f64::from_bits(0x7ff4000000000000);
            let q_nan = f64::from_bits(0x7ff8000000000000);
            check_lt(s_nan, q_nan);

            let neg_s_nan = f64::from_bits(0xfff4000000000000);
            let neg_q_nan = f64::from_bits(0xfff8000000000000);
            check_lt(neg_q_nan, neg_s_nan);

            let s_nan = f32::from_bits(0x7fa00000);
            let q_nan = f32::from_bits(0x7fc00000);
            check_lt(s_nan, q_nan);

            let neg_s_nan = f32::from_bits(0xffa00000);
            let neg_q_nan = f32::from_bits(0xffc00000);
            check_lt(neg_q_nan, neg_s_nan);
        }

        check_lt(-f64::NAN, f64::NEG_INFINITY);
        check_gt(1.0_f64, -f64::NAN);
        check_lt(f64::INFINITY, f64::NAN);
        check_gt(f64::NAN, 1.0_f64);

        check_lt(-f32::NAN, f32::NEG_INFINITY);
        check_gt(1.0_f32, -f32::NAN);
        check_lt(f32::INFINITY, f32::NAN);
        check_gt(f32::NAN, 1.0_f32);
    }
}
