#[cfg(test)]
mod tests {
    use crate::int::PrimInt;

    #[test]
    pub fn reverse_bits() {

        assert_eq!(
            PrimInt::reverse_bits(0x0123_4567_89ab_cdefu64),
            0xf7b3_d591_e6a2_c480
        );

        assert_eq!(PrimInt::reverse_bits(0i8), 0);
        assert_eq!(PrimInt::reverse_bits(-1i8), -1);
        assert_eq!(PrimInt::reverse_bits(1i8), i8::MIN);
        assert_eq!(PrimInt::reverse_bits(i8::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(-2i8), i8::MAX);
        assert_eq!(PrimInt::reverse_bits(i8::MAX), -2);

        assert_eq!(PrimInt::reverse_bits(0i16), 0);
        assert_eq!(PrimInt::reverse_bits(-1i16), -1);
        assert_eq!(PrimInt::reverse_bits(1i16), i16::MIN);
        assert_eq!(PrimInt::reverse_bits(i16::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(-2i16), i16::MAX);
        assert_eq!(PrimInt::reverse_bits(i16::MAX), -2);

        assert_eq!(PrimInt::reverse_bits(0i32), 0);
        assert_eq!(PrimInt::reverse_bits(-1i32), -1);
        assert_eq!(PrimInt::reverse_bits(1i32), i32::MIN);
        assert_eq!(PrimInt::reverse_bits(i32::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(-2i32), i32::MAX);
        assert_eq!(PrimInt::reverse_bits(i32::MAX), -2);

        assert_eq!(PrimInt::reverse_bits(0i64), 0);
        assert_eq!(PrimInt::reverse_bits(-1i64), -1);
        assert_eq!(PrimInt::reverse_bits(1i64), i64::MIN);
        assert_eq!(PrimInt::reverse_bits(i64::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(-2i64), i64::MAX);
        assert_eq!(PrimInt::reverse_bits(i64::MAX), -2);
    }

    #[test]
    pub fn reverse_bits_i128() {

        assert_eq!(PrimInt::reverse_bits(0i128), 0);
        assert_eq!(PrimInt::reverse_bits(-1i128), -1);
        assert_eq!(PrimInt::reverse_bits(1i128), i128::MIN);
        assert_eq!(PrimInt::reverse_bits(i128::MIN), 1);
        assert_eq!(PrimInt::reverse_bits(-2i128), i128::MAX);
        assert_eq!(PrimInt::reverse_bits(i128::MAX), -2);
    }
}
