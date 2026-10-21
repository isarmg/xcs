#[cfg(test)]
mod tests {
    use crate::{consts::*, Integer, ToInt};

    #[test]
    fn to_ix_min() {
        assert_eq!(N128::to_i8(), ::core::i8::MIN);
        assert_eq!(N32768::to_i16(), ::core::i16::MIN);
    }

    #[test]
    fn int_toint_test() {
        // i8
        assert_eq!(0_i8, Z0::to_int());
        assert_eq!(1_i8, P1::to_int());
        assert_eq!(2_i8, P2::to_int());
        assert_eq!(3_i8, P3::to_int());
        assert_eq!(4_i8, P4::to_int());
        assert_eq!(-1_i8, N1::to_int());
        assert_eq!(-2_i8, N2::to_int());
        assert_eq!(-3_i8, N3::to_int());
        assert_eq!(-4_i8, N4::to_int());
        assert_eq!(0_i8, Z0::INT);
        assert_eq!(1_i8, P1::INT);
        assert_eq!(2_i8, P2::INT);
        assert_eq!(3_i8, P3::INT);
        assert_eq!(4_i8, P4::INT);
        assert_eq!(-1_i8, N1::INT);
        assert_eq!(-2_i8, N2::INT);
        assert_eq!(-3_i8, N3::INT);
        assert_eq!(-4_i8, N4::INT);

        // i16
        assert_eq!(0_i16, Z0::to_int());
        assert_eq!(1_i16, P1::to_int());
        assert_eq!(2_i16, P2::to_int());
        assert_eq!(3_i16, P3::to_int());
        assert_eq!(4_i16, P4::to_int());
        assert_eq!(-1_i16, N1::to_int());
        assert_eq!(-2_i16, N2::to_int());
        assert_eq!(-3_i16, N3::to_int());
        assert_eq!(-4_i16, N4::to_int());
        assert_eq!(0_i16, Z0::INT);
        assert_eq!(1_i16, P1::INT);
        assert_eq!(2_i16, P2::INT);
        assert_eq!(3_i16, P3::INT);
        assert_eq!(4_i16, P4::INT);
        assert_eq!(-1_i16, N1::INT);
        assert_eq!(-2_i16, N2::INT);
        assert_eq!(-3_i16, N3::INT);
        assert_eq!(-4_i16, N4::INT);

        // i32
        assert_eq!(0_i32, Z0::to_int());
        assert_eq!(1_i32, P1::to_int());
        assert_eq!(2_i32, P2::to_int());
        assert_eq!(3_i32, P3::to_int());
        assert_eq!(4_i32, P4::to_int());
        assert_eq!(-1_i32, N1::to_int());
        assert_eq!(-2_i32, N2::to_int());
        assert_eq!(-3_i32, N3::to_int());
        assert_eq!(-4_i32, N4::to_int());
        assert_eq!(0_i32, Z0::INT);
        assert_eq!(1_i32, P1::INT);
        assert_eq!(2_i32, P2::INT);
        assert_eq!(3_i32, P3::INT);
        assert_eq!(4_i32, P4::INT);
        assert_eq!(-1_i32, N1::INT);
        assert_eq!(-2_i32, N2::INT);
        assert_eq!(-3_i32, N3::INT);
        assert_eq!(-4_i32, N4::INT);

        // i64
        assert_eq!(0_i64, Z0::to_int());
        assert_eq!(1_i64, P1::to_int());
        assert_eq!(2_i64, P2::to_int());
        assert_eq!(3_i64, P3::to_int());
        assert_eq!(4_i64, P4::to_int());
        assert_eq!(-1_i64, N1::to_int());
        assert_eq!(-2_i64, N2::to_int());
        assert_eq!(-3_i64, N3::to_int());
        assert_eq!(-4_i64, N4::to_int());
        assert_eq!(0_i64, Z0::INT);
        assert_eq!(1_i64, P1::INT);
        assert_eq!(2_i64, P2::INT);
        assert_eq!(3_i64, P3::INT);
        assert_eq!(4_i64, P4::INT);
        assert_eq!(-1_i64, N1::INT);
        assert_eq!(-2_i64, N2::INT);
        assert_eq!(-3_i64, N3::INT);
        assert_eq!(-4_i64, N4::INT);

        // isize
        assert_eq!(0_isize, Z0::to_int());
        assert_eq!(1_isize, P1::to_int());
        assert_eq!(2_isize, P2::to_int());
        assert_eq!(3_isize, P3::to_int());
        assert_eq!(4_isize, P4::to_int());
        assert_eq!(-1_isize, N1::to_int());
        assert_eq!(-2_isize, N2::to_int());
        assert_eq!(-3_isize, N3::to_int());
        assert_eq!(-4_isize, N4::to_int());
        assert_eq!(0_isize, Z0::INT);
        assert_eq!(1_isize, P1::INT);
        assert_eq!(2_isize, P2::INT);
        assert_eq!(3_isize, P3::INT);
        assert_eq!(4_isize, P4::INT);
        assert_eq!(-1_isize, N1::INT);
        assert_eq!(-2_isize, N2::INT);
        assert_eq!(-3_isize, N3::INT);
        assert_eq!(-4_isize, N4::INT);

        // i128
        #[cfg(feature = "i128")]
        {
            assert_eq!(0_i128, Z0::to_int());
            assert_eq!(1_i128, P1::to_int());
            assert_eq!(2_i128, P2::to_int());
            assert_eq!(3_i128, P3::to_int());
            assert_eq!(4_i128, P4::to_int());
            assert_eq!(-1_i128, N1::to_int());
            assert_eq!(-2_i128, N2::to_int());
            assert_eq!(-3_i128, N3::to_int());
            assert_eq!(-4_i128, N4::to_int());
            assert_eq!(0_i128, Z0::INT);
            assert_eq!(1_i128, P1::INT);
            assert_eq!(2_i128, P2::INT);
            assert_eq!(3_i128, P3::INT);
            assert_eq!(4_i128, P4::INT);
            assert_eq!(-1_i128, N1::INT);
            assert_eq!(-2_i128, N2::INT);
            assert_eq!(-3_i128, N3::INT);
            assert_eq!(-4_i128, N4::INT);
        }
    }
}
