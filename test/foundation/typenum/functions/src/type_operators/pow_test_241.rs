#[test]
fn pow_test() {
    use crate::consts::*;
    let z0 = Z0::new();
    let p3 = P3::new();

    let u0 = U0::new();
    let u3 = U3::new();
    let n3 = N3::new();

    macro_rules! check {
        ($x:ident) => {
            assert_eq!($x.powi(z0), 1);
            assert_eq!($x.powi(u0), 1);

            assert_eq!($x.powi(p3), $x * $x * $x);
            assert_eq!($x.powi(u3), $x * $x * $x);
        };
        ($x:ident, $f:ident) => {
            assert!((<$f as Pow<Z0>>::powi(*$x, z0) - 1.0).abs() < ::core::$f::EPSILON);
            assert!((<$f as Pow<U0>>::powi(*$x, u0) - 1.0).abs() < ::core::$f::EPSILON);

            assert!((<$f as Pow<P3>>::powi(*$x, p3) - $x * $x * $x).abs() < ::core::$f::EPSILON);
            assert!((<$f as Pow<U3>>::powi(*$x, u3) - $x * $x * $x).abs() < ::core::$f::EPSILON);

            if *$x == 0.0 {
                assert!(<$f as Pow<N3>>::powi(*$x, n3).is_infinite());
            } else {
                assert!(
                    (<$f as Pow<N3>>::powi(*$x, n3) - 1. / $x / $x / $x).abs()
                        < ::core::$f::EPSILON
                );
            }
        };
    }

    for x in &[0i8, -3, 2] {
        check!(x);
    }
    for x in &[0u8, 1, 5] {
        check!(x);
    }
    for x in &[0usize, 1, 5, 40] {
        check!(x);
    }
    for x in &[0isize, 1, 2, -30, -22, 48] {
        check!(x);
    }
    for x in &[0.0f32, 2.2, -3.5, 378.223] {
        check!(x, f32);
    }
    for x in &[0.0f64, 2.2, -3.5, -2387.2, 234.22] {
        check!(x, f64);
    }
}
