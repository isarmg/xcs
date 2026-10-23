#[cfg(test)]
mod tests {
    use super::Choice;
    use crate::{CtEq, CtGt, CtLt, CtSelect};

    #[test]
    fn ct_eq() {
        let a = Choice::TRUE;
        let b = Choice::TRUE;
        let c = Choice::FALSE;

        assert!(a.ct_eq(&b).to_bool());
        assert!(!a.ct_eq(&c).to_bool());
        assert!(!b.ct_eq(&c).to_bool());

        assert!(!a.ct_ne(&b).to_bool());
        assert!(a.ct_ne(&c).to_bool());
        assert!(b.ct_ne(&c).to_bool());
    }

    #[test]
    fn ct_select() {
        let a = Choice::FALSE;
        let b = Choice::TRUE;
        assert_eq!(a.ct_select(&b, Choice::FALSE).to_bool(), a.to_bool());
        assert_eq!(a.ct_select(&b, Choice::TRUE).to_bool(), b.to_bool());
    }

    #[test]
    fn and() {
        assert_eq!((Choice::FALSE & Choice::FALSE).to_u8(), 0);
        assert_eq!((Choice::TRUE & Choice::FALSE).to_u8(), 0);
        assert_eq!((Choice::FALSE & Choice::TRUE).to_u8(), 0);
        assert_eq!((Choice::TRUE & Choice::TRUE).to_u8(), 1);
    }

    #[test]
    fn or() {
        assert_eq!((Choice::FALSE | Choice::FALSE).to_u8(), 0);
        assert_eq!((Choice::TRUE | Choice::FALSE).to_u8(), 1);
        assert_eq!((Choice::FALSE | Choice::TRUE).to_u8(), 1);
        assert_eq!((Choice::TRUE | Choice::TRUE).to_u8(), 1);
    }

    #[test]
    fn xor() {
        assert_eq!((Choice::FALSE ^ Choice::FALSE).to_u8(), 0);
        assert_eq!((Choice::TRUE ^ Choice::FALSE).to_u8(), 1);
        assert_eq!((Choice::FALSE ^ Choice::TRUE).to_u8(), 1);
        assert_eq!((Choice::TRUE ^ Choice::TRUE).to_u8(), 0);
    }

    #[test]
    fn not() {
        assert_eq!(Choice::FALSE.not().to_u8(), 1);
        assert_eq!(Choice::TRUE.not().to_u8(), 0);
    }

    #[test]
    fn from_i64_eq() {
        assert!(Choice::from_i64_eq(0, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_i64_eq(1, 1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u8_eq() {
        assert!(Choice::from_u8_eq(0, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_eq(1, 1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u8_le() {
        assert!(Choice::from_u8_le(0, 0).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u8_le(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_le(1, 1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u8_le(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u8_lsb() {
        assert!(Choice::from_u8_lsb(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_lsb(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u8_lsb(2).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_lsb(3).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u8_lt() {
        assert!(Choice::from_u8_lt(0, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_lt(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_lt(1, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_lt(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u8_nz() {
        assert!(Choice::from_u8_nz(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u8_nz(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u8_nz(2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u16_eq() {
        assert!(Choice::from_u16_eq(0, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_eq(1, 1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u16_le() {
        assert!(Choice::from_u16_le(0, 0).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u16_le(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_le(1, 1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u16_le(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u16_lsb() {
        assert!(Choice::from_u16_lsb(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_lsb(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u16_lsb(2).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_lsb(3).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u16_lt() {
        assert!(Choice::from_u16_lt(0, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_lt(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_lt(1, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_lt(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u16_nz() {
        assert!(Choice::from_u16_nz(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u16_nz(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u16_nz(2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u32_eq() {
        assert!(Choice::from_u32_eq(0, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_eq(1, 1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u32_le() {
        assert!(Choice::from_u32_le(0, 0).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u32_le(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_le(1, 1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u32_le(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u32_lsb() {
        assert!(Choice::from_u32_lsb(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_lsb(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u32_lsb(2).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_lsb(3).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u32_lt() {
        assert!(Choice::from_u32_lt(0, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_lt(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_lt(1, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_lt(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u32_nz() {
        assert!(Choice::from_u32_nz(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u32_nz(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u32_nz(2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u64_eq() {
        assert!(Choice::from_u64_eq(0, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_eq(1, 1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u64_le() {
        assert!(Choice::from_u64_le(0, 0).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u64_le(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_le(1, 1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u64_le(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u64_lsb() {
        assert!(Choice::from_u64_lsb(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_lsb(1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u64_lt() {
        assert!(Choice::from_u64_lt(0, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_lt(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_lt(1, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_lt(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u64_nz() {
        assert!(Choice::from_u64_nz(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u64_nz(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u64_nz(2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u128_eq() {
        assert!(Choice::from_u128_eq(0, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_eq(1, 1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u128_le() {
        assert!(Choice::from_u128_le(0, 0).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u128_le(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_le(1, 1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u128_le(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u128_lsb() {
        assert!(Choice::from_u128_lsb(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_lsb(1).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u128_lt() {
        assert!(Choice::from_u128_lt(0, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_lt(1, 0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_lt(1, 1).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_lt(1, 2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn from_u128_comparison_boundaries() {
        const LT: Choice = Choice::from_u128_lt_halves((1 << 96) - 1, 1 << 96);
        const LE: Choice = Choice::from_u128_le(1 << 64, 1 << 64);
        assert!(LT.to_bool());
        assert!(LE.to_bool());

        let values = [
            0,
            1,
            (1 << 32) - 1,
            1 << 32,
            (1 << 64) - 1,
            1 << 64,
            (1 << 64) + 1,
            (1 << 96) - 1,
            1 << 96,
            u128::MAX - 1,
            u128::MAX,
        ];

        for x in values {
            for y in values {
                assert_eq!(x < y, Choice::from_u128_lt_halves(x, y).to_bool());
                assert_eq!(x < y, Choice::from_u128_lt(x, y).to_bool());
                assert_eq!(x <= y, Choice::from_u128_le(x, y).to_bool());
                assert_eq!(x < y, x.ct_lt(&y).to_bool());
                assert_eq!(x > y, x.ct_gt(&y).to_bool());
            }
        }
    }

    #[test]
    fn from_u128_nz() {
        assert!(Choice::from_u128_nz(0).eq(Choice::FALSE).to_bool());
        assert!(Choice::from_u128_nz(1).eq(Choice::TRUE).to_bool());
        assert!(Choice::from_u128_nz(2).eq(Choice::TRUE).to_bool());
    }

    #[test]
    fn select_i64() {
        let a: i64 = 1;
        let b: i64 = 2;
        assert_eq!(Choice::TRUE.select_i64(a, b), b);
        assert_eq!(Choice::FALSE.select_i64(a, b), a);
    }

    #[test]
    fn select_u8() {
        let a: u8 = 1;
        let b: u8 = 2;
        assert_eq!(Choice::TRUE.select_u8(a, b), b);
        assert_eq!(Choice::FALSE.select_u8(a, b), a);
    }

    #[test]
    fn select_u16() {
        let a: u16 = 1;
        let b: u16 = 2;
        assert_eq!(Choice::TRUE.select_u16(a, b), b);
        assert_eq!(Choice::FALSE.select_u16(a, b), a);
    }

    #[test]
    fn select_u32() {
        let a: u32 = 1;
        let b: u32 = 2;
        assert_eq!(Choice::TRUE.select_u32(a, b), b);
        assert_eq!(Choice::FALSE.select_u32(a, b), a);
    }

    #[test]
    fn select_u64() {
        let a: u64 = 1;
        let b: u64 = 2;
        assert_eq!(Choice::TRUE.select_u64(a, b), b);
        assert_eq!(Choice::FALSE.select_u64(a, b), a);
    }

    #[test]
    fn select_u128() {
        let a: u128 = 1;
        let b: u128 = 2;
        assert_eq!(Choice::TRUE.select_u128(a, b), b);
        assert_eq!(Choice::FALSE.select_u128(a, b), a);
    }

    #[test]
    fn to_bool() {
        assert!(!Choice::FALSE.to_bool());
        assert!(Choice::TRUE.to_bool());
    }

    #[test]
    fn to_u8() {
        assert_eq!(Choice::FALSE.to_u8(), 0);
        assert_eq!(Choice::TRUE.to_u8(), 1);
    }

    #[test]
    fn to_u8_mask() {
        assert_eq!(Choice::FALSE.to_u8_mask(), 0);
        assert_eq!(Choice::TRUE.to_u8_mask(), u8::MAX);
    }

    #[test]
    fn to_u16_mask() {
        assert_eq!(Choice::FALSE.to_u16_mask(), 0);
        assert_eq!(Choice::TRUE.to_u16_mask(), u16::MAX);
    }

    #[test]
    fn to_u32_mask() {
        assert_eq!(Choice::FALSE.to_u32_mask(), 0);
        assert_eq!(Choice::TRUE.to_u32_mask(), u32::MAX);
    }

    #[test]
    fn to_u64_mask() {
        assert_eq!(Choice::FALSE.to_u64_mask(), 0);
        assert_eq!(Choice::TRUE.to_u64_mask(), u64::MAX);
    }

    #[test]
    fn to_u128_mask() {
        assert_eq!(Choice::FALSE.to_u128_mask(), 0);
        assert_eq!(Choice::TRUE.to_u128_mask(), u128::MAX);
    }
}
