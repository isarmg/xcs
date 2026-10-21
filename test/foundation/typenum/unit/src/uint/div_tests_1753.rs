#[cfg(test)]
mod div_tests {
    use crate::Unsigned;

    use super::SetBitOut;

    macro_rules! test_div {
        ($a:ident / $b:ident = $c:ident) => {{
            type R = Quot<$a, $b>;
            assert_eq!(<R as Unsigned>::to_usize(), $c::to_usize());
        }};
    }
    #[test]
    fn test_div() {
        use crate::consts::*;
        use crate::{Quot, Same};

        test_div!(U0 / U1 = U0);
        test_div!(U1 / U1 = U1);
        test_div!(U2 / U1 = U2);
        test_div!(U3 / U1 = U3);
        test_div!(U4 / U1 = U4);

        test_div!(U0 / U2 = U0);
        test_div!(U1 / U2 = U0);
        test_div!(U2 / U2 = U1);
        test_div!(U3 / U2 = U1);
        test_div!(U4 / U2 = U2);
        test_div!(U6 / U2 = U3);
        test_div!(U7 / U2 = U3);

        type T = <SetBitOut<U0, U1, B1> as Same<U2>>::Output;
        <T as Unsigned>::to_u32();
    }
}
