#[cfg(test)]
mod test {
    use crate::complex::{
        complex_float::ComplexFloat,
        test::{_0_0i, _0_1i, _1_0i, _1_1i, float::close},
        Complex,
    };
    use std::f64; // for constants before Rust 1.43.

    fn closef(a: f64, b: f64) -> bool {
        close_to_tolf(a, b, 1e-10)
    }

    fn close_to_tolf(a: f64, b: f64, tol: f64) -> bool {
        // returns true if a and b are reasonably close
        let close = (a == b) || (a - b).abs() < tol;
        if !close {
            println!("{:?} != {:?}", a, b);
        }
        close
    }

    #[test]
    fn test_exp2() {
        assert!(close(ComplexFloat::exp2(_0_0i), _1_0i));
        assert!(closef(<f64 as ComplexFloat>::exp2(0.), 1.));
    }

    #[test]
    fn test_exp() {
        assert!(close(ComplexFloat::exp(_0_0i), _1_0i));
        assert!(closef(ComplexFloat::exp(0.), 1.));
    }

    #[test]
    fn test_powi() {
        assert!(close(ComplexFloat::powi(_0_1i, 4), _1_0i));
        assert!(closef(ComplexFloat::powi(-1., 4), 1.));
    }

    #[test]
    fn test_powz() {
        assert!(close(ComplexFloat::powc(_1_0i, _0_1i), _1_0i));
        assert!(close(ComplexFloat::powc(1., _0_1i), _1_0i));
    }

    #[test]
    fn test_log2() {
        assert!(close(ComplexFloat::log2(_1_0i), _0_0i));
        assert!(closef(ComplexFloat::log2(1.), 0.));
    }

    #[test]
    fn test_log10() {
        assert!(close(ComplexFloat::log10(_1_0i), _0_0i));
        assert!(closef(ComplexFloat::log10(1.), 0.));
    }

    #[test]
    fn test_conj() {
        assert_eq!(ComplexFloat::conj(_0_1i), Complex::new(0., -1.));
        assert_eq!(ComplexFloat::conj(1.), 1.);
    }

    #[test]
    fn test_is_nan() {
        assert!(!ComplexFloat::is_nan(_1_0i));
        assert!(!ComplexFloat::is_nan(1.));

        assert!(ComplexFloat::is_nan(Complex::new(f64::NAN, f64::NAN)));
        assert!(ComplexFloat::is_nan(f64::NAN));
    }

    #[test]
    fn test_is_infinite() {
        assert!(!ComplexFloat::is_infinite(_1_0i));
        assert!(!ComplexFloat::is_infinite(1.));

        assert!(ComplexFloat::is_infinite(Complex::new(
            f64::INFINITY,
            f64::INFINITY
        )));
        assert!(ComplexFloat::is_infinite(f64::INFINITY));
    }

    #[test]
    fn test_is_finite() {
        assert!(ComplexFloat::is_finite(_1_0i));
        assert!(ComplexFloat::is_finite(1.));

        assert!(!ComplexFloat::is_finite(Complex::new(
            f64::INFINITY,
            f64::INFINITY
        )));
        assert!(!ComplexFloat::is_finite(f64::INFINITY));
    }

    #[test]
    fn test_is_normal() {
        assert!(ComplexFloat::is_normal(_1_1i));
        assert!(ComplexFloat::is_normal(1.));

        assert!(!ComplexFloat::is_normal(Complex::new(
            f64::INFINITY,
            f64::INFINITY
        )));
        assert!(!ComplexFloat::is_normal(f64::INFINITY));
    }

    #[test]
    fn test_arg() {
        assert!(closef(
            ComplexFloat::arg(_0_1i),
            core::f64::consts::FRAC_PI_2
        ));

        assert!(closef(ComplexFloat::arg(-1.), core::f64::consts::PI));
        assert!(closef(ComplexFloat::arg(-0.), core::f64::consts::PI));
        assert!(closef(ComplexFloat::arg(0.), 0.));
        assert!(closef(ComplexFloat::arg(1.), 0.));
        assert!(ComplexFloat::arg(f64::NAN).is_nan());
    }
}
