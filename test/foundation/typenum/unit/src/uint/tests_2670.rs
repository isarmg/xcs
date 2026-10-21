#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::{Log2, ToInt, Unsigned};

    #[test]
    fn log2_test() {
        assert_eq!(0, <Log2<U1>>::to_u32());

        assert_eq!(1, <Log2<U2>>::to_u32());
        assert_eq!(1, <Log2<U3>>::to_u32());

        assert_eq!(2, <Log2<U4>>::to_u32());
        assert_eq!(2, <Log2<U5>>::to_u32());
        assert_eq!(2, <Log2<U6>>::to_u32());
        assert_eq!(2, <Log2<U7>>::to_u32());

        assert_eq!(3, <Log2<U8>>::to_u32());
        assert_eq!(3, <Log2<U9>>::to_u32());
        assert_eq!(3, <Log2<U10>>::to_u32());
        assert_eq!(3, <Log2<U11>>::to_u32());
        assert_eq!(3, <Log2<U12>>::to_u32());
        assert_eq!(3, <Log2<U13>>::to_u32());
        assert_eq!(3, <Log2<U14>>::to_u32());
        assert_eq!(3, <Log2<U15>>::to_u32());

        assert_eq!(4, <Log2<U16>>::to_u32());
        assert_eq!(4, <Log2<U17>>::to_u32());
        assert_eq!(4, <Log2<U18>>::to_u32());
        assert_eq!(4, <Log2<U19>>::to_u32());
        assert_eq!(4, <Log2<U20>>::to_u32());
        assert_eq!(4, <Log2<U21>>::to_u32());
        assert_eq!(4, <Log2<U22>>::to_u32());
        assert_eq!(4, <Log2<U23>>::to_u32());
        assert_eq!(4, <Log2<U24>>::to_u32());
        assert_eq!(4, <Log2<U25>>::to_u32());
        assert_eq!(4, <Log2<U26>>::to_u32());
        assert_eq!(4, <Log2<U27>>::to_u32());
        assert_eq!(4, <Log2<U28>>::to_u32());
        assert_eq!(4, <Log2<U29>>::to_u32());
        assert_eq!(4, <Log2<U30>>::to_u32());
        assert_eq!(4, <Log2<U31>>::to_u32());

        assert_eq!(5, <Log2<U32>>::to_u32());
        assert_eq!(5, <Log2<U33>>::to_u32());

        // ...
    }

    #[test]
    fn uint_toint_test() {
        // i8
        assert_eq!(0_i8, U0::to_int());
        assert_eq!(1_i8, U1::to_int());
        assert_eq!(2_i8, U2::to_int());
        assert_eq!(3_i8, U3::to_int());
        assert_eq!(4_i8, U4::to_int());
        assert_eq!(0_i8, U0::INT);
        assert_eq!(1_i8, U1::INT);
        assert_eq!(2_i8, U2::INT);
        assert_eq!(3_i8, U3::INT);
        assert_eq!(4_i8, U4::INT);

        // i16
        assert_eq!(0_i16, U0::to_int());
        assert_eq!(1_i16, U1::to_int());
        assert_eq!(2_i16, U2::to_int());
        assert_eq!(3_i16, U3::to_int());
        assert_eq!(4_i16, U4::to_int());
        assert_eq!(0_i16, U0::INT);
        assert_eq!(1_i16, U1::INT);
        assert_eq!(2_i16, U2::INT);
        assert_eq!(3_i16, U3::INT);
        assert_eq!(4_i16, U4::INT);

        // i32
        assert_eq!(0_i32, U0::to_int());
        assert_eq!(1_i32, U1::to_int());
        assert_eq!(2_i32, U2::to_int());
        assert_eq!(3_i32, U3::to_int());
        assert_eq!(4_i32, U4::to_int());
        assert_eq!(0_i32, U0::INT);
        assert_eq!(1_i32, U1::INT);
        assert_eq!(2_i32, U2::INT);
        assert_eq!(3_i32, U3::INT);
        assert_eq!(4_i32, U4::INT);

        // i64
        assert_eq!(0_i64, U0::to_int());
        assert_eq!(1_i64, U1::to_int());
        assert_eq!(2_i64, U2::to_int());
        assert_eq!(3_i64, U3::to_int());
        assert_eq!(4_i64, U4::to_int());
        assert_eq!(0_i64, U0::INT);
        assert_eq!(1_i64, U1::INT);
        assert_eq!(2_i64, U2::INT);
        assert_eq!(3_i64, U3::INT);
        assert_eq!(4_i64, U4::INT);

        // isize
        assert_eq!(0_isize, U0::to_int());
        assert_eq!(1_isize, U1::to_int());
        assert_eq!(2_isize, U2::to_int());
        assert_eq!(3_isize, U3::to_int());
        assert_eq!(4_isize, U4::to_int());
        assert_eq!(0_isize, U0::INT);
        assert_eq!(1_isize, U1::INT);
        assert_eq!(2_isize, U2::INT);
        assert_eq!(3_isize, U3::INT);
        assert_eq!(4_isize, U4::INT);

        // i128
        #[cfg(feature = "i128")]
        {
            assert_eq!(0_i128, U0::to_int());
            assert_eq!(1_i128, U1::to_int());
            assert_eq!(2_i128, U2::to_int());
            assert_eq!(3_i128, U3::to_int());
            assert_eq!(4_i128, U4::to_int());
            assert_eq!(0_i128, U0::INT);
            assert_eq!(1_i128, U1::INT);
            assert_eq!(2_i128, U2::INT);
            assert_eq!(3_i128, U3::INT);
            assert_eq!(4_i128, U4::INT);
        }

        // u8
        assert_eq!(0_u8, U0::to_int());
        assert_eq!(1_u8, U1::to_int());
        assert_eq!(2_u8, U2::to_int());
        assert_eq!(3_u8, U3::to_int());
        assert_eq!(4_u8, U4::to_int());
        assert_eq!(0_u8, U0::INT);
        assert_eq!(1_u8, U1::INT);
        assert_eq!(2_u8, U2::INT);
        assert_eq!(3_u8, U3::INT);
        assert_eq!(4_u8, U4::INT);

        // u16
        assert_eq!(0_u16, U0::to_int());
        assert_eq!(1_u16, U1::to_int());
        assert_eq!(2_u16, U2::to_int());
        assert_eq!(3_u16, U3::to_int());
        assert_eq!(4_u16, U4::to_int());
        assert_eq!(0_u16, U0::INT);
        assert_eq!(1_u16, U1::INT);
        assert_eq!(2_u16, U2::INT);
        assert_eq!(3_u16, U3::INT);
        assert_eq!(4_u16, U4::INT);

        // u32
        assert_eq!(0_u32, U0::to_int());
        assert_eq!(1_u32, U1::to_int());
        assert_eq!(2_u32, U2::to_int());
        assert_eq!(3_u32, U3::to_int());
        assert_eq!(4_u32, U4::to_int());
        assert_eq!(0_u32, U0::INT);
        assert_eq!(1_u32, U1::INT);
        assert_eq!(2_u32, U2::INT);
        assert_eq!(3_u32, U3::INT);
        assert_eq!(4_u32, U4::INT);

        // u64
        assert_eq!(0_u64, U0::to_int());
        assert_eq!(1_u64, U1::to_int());
        assert_eq!(2_u64, U2::to_int());
        assert_eq!(3_u64, U3::to_int());
        assert_eq!(4_u64, U4::to_int());
        assert_eq!(0_u64, U0::INT);
        assert_eq!(1_u64, U1::INT);
        assert_eq!(2_u64, U2::INT);
        assert_eq!(3_u64, U3::INT);
        assert_eq!(4_u64, U4::INT);

        // usize
        assert_eq!(0_usize, U0::to_int());
        assert_eq!(1_usize, U1::to_int());
        assert_eq!(2_usize, U2::to_int());
        assert_eq!(3_usize, U3::to_int());
        assert_eq!(4_usize, U4::to_int());
        assert_eq!(0_usize, U0::INT);
        assert_eq!(1_usize, U1::INT);
        assert_eq!(2_usize, U2::INT);
        assert_eq!(3_usize, U3::INT);
        assert_eq!(4_usize, U4::INT);

        // u128
        #[cfg(feature = "i128")]
        {
            assert_eq!(0_u128, U0::to_int());
            assert_eq!(1_u128, U1::to_int());
            assert_eq!(2_u128, U2::to_int());
            assert_eq!(3_u128, U3::to_int());
            assert_eq!(4_u128, U4::to_int());
            assert_eq!(0_u128, U0::INT);
            assert_eq!(1_u128, U1::INT);
            assert_eq!(2_u128, U2::INT);
            assert_eq!(3_u128, U3::INT);
            assert_eq!(4_u128, U4::INT);
        }
    }
}
