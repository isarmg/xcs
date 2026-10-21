#[cfg(test)]
mod gcd_tests {
    use super::*;
    use crate::consts::*;

    macro_rules! gcd_test {
        (
            $( $a:ident, $b:ident => $c:ident ),* $(,)*
        ) => {
            $(
                assert_eq!(<Gcf<$a, $b> as Unsigned>::to_usize(), $c::to_usize());
                assert_eq!(<Gcf<$b, $a> as Unsigned>::to_usize(), $c::to_usize());
             )*
        }
    }

    #[test]
    fn gcd() {
        gcd_test! {
            U0,   U0    => U0,
            U0,   U42   => U42,
            U12,  U8    => U4,
            U13,  U1013 => U1,  // Two primes
            U9,   U26   => U1,  // Not prime but coprime
            U143, U273  => U13,
            U117, U273  => U39,
        }
    }
}
