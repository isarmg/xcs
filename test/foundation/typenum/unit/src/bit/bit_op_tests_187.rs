#[cfg(test)]
mod bit_op_tests {
    use core::ops::{BitAnd, BitOr, BitXor, Not};

    use crate::{B0, B1};

    // macro for testing operation results. Uses `Same` to ensure the types are equal and
    // not just the values they evaluate to.
    macro_rules! test_bit_op {
        ($op:ident $Lhs:ident = $Answer:ident) => {{
            type Test = <<$Lhs as $op>::Output as $crate::Same<$Answer>>::Output;
            assert_eq!(
                <$Answer as $crate::Bit>::to_u8(),
                <Test as $crate::Bit>::to_u8()
            );
        }};
        ($Lhs:ident $op:ident $Rhs:ident = $Answer:ident) => {{
            type Test = <<$Lhs as $op<$Rhs>>::Output as $crate::Same<$Answer>>::Output;
            assert_eq!(
                <$Answer as $crate::Bit>::to_u8(),
                <Test as $crate::Bit>::to_u8()
            );
        }};
    }

    #[test]
    fn bit_operations() {
        test_bit_op!(Not B0 = B1);
        test_bit_op!(Not B1 = B0);

        test_bit_op!(B0 BitAnd B0 = B0);
        test_bit_op!(B0 BitAnd B1 = B0);
        test_bit_op!(B1 BitAnd B0 = B0);
        test_bit_op!(B1 BitAnd B1 = B1);

        test_bit_op!(B0 BitOr B0 = B0);
        test_bit_op!(B0 BitOr B1 = B1);
        test_bit_op!(B1 BitOr B0 = B1);
        test_bit_op!(B1 BitOr B1 = B1);

        test_bit_op!(B0 BitXor B0 = B0);
        test_bit_op!(B0 BitXor B1 = B1);
        test_bit_op!(B1 BitXor B0 = B1);
        test_bit_op!(B1 BitXor B1 = B0);
    }
}
