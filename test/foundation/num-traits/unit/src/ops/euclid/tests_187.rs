#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclid_unsigned() {
        macro_rules! test_euclid {
            ($($t:ident)+) => {
                $(
                    {
                        let x: $t = 10;
                        let y: $t = 3;
                        let div = Euclid::div_euclid(&x, &y);
                        let rem = Euclid::rem_euclid(&x, &y);
                        assert_eq!(div, 3);
                        assert_eq!(rem, 1);
                        assert_eq!((div, rem), Euclid::div_rem_euclid(&x, &y));
                    }
                )+
            };
        }

        test_euclid!(usize u8 u16 u32 u64);
    }

    #[test]
    fn euclid_signed() {
        macro_rules! test_euclid {
            ($($t:ident)+) => {
                $(
                    {
                        let x: $t = 10;
                        let y: $t = -3;
                        assert_eq!(Euclid::div_euclid(&x, &y), -3);
                        assert_eq!(Euclid::div_euclid(&-x, &y), 4);
                        assert_eq!(Euclid::rem_euclid(&x, &y), 1);
                        assert_eq!(Euclid::rem_euclid(&-x, &y), 2);
                        assert_eq!((Euclid::div_euclid(&x, &y), Euclid::rem_euclid(&x, &y)), Euclid::div_rem_euclid(&x, &y));
                        let x: $t = $t::MIN + 1;
                        let y: $t = -1;
                        assert_eq!(Euclid::div_euclid(&x, &y), $t::MAX);
                    }
                )+
            };
        }

        test_euclid!(isize i8 i16 i32 i64 i128);
    }

    #[test]
    fn euclid_float() {
        macro_rules! test_euclid {
            ($($t:ident)+) => {
                $(
                    {
                        let x: $t = 12.1;
                        let y: $t = 3.2;
                        assert!(Euclid::div_euclid(&x, &y) * y + Euclid::rem_euclid(&x, &y) - x
                        <= 46.4 * <$t as crate::float::FloatCore>::epsilon());
                        assert!(Euclid::div_euclid(&x, &-y) * -y + Euclid::rem_euclid(&x, &-y) - x
                        <= 46.4 * <$t as crate::float::FloatCore>::epsilon());
                        assert!(Euclid::div_euclid(&-x, &y) * y + Euclid::rem_euclid(&-x, &y) + x
                        <= 46.4 * <$t as crate::float::FloatCore>::epsilon());
                        assert!(Euclid::div_euclid(&-x, &-y) * -y + Euclid::rem_euclid(&-x, &-y) + x
                        <= 46.4 * <$t as crate::float::FloatCore>::epsilon());
                        assert_eq!((Euclid::div_euclid(&x, &y), Euclid::rem_euclid(&x, &y)), Euclid::div_rem_euclid(&x, &y));
                    }
                )+
            };
        }

        test_euclid!(f32 f64);
    }

    #[test]
    fn euclid_checked() {
        macro_rules! test_euclid_checked {
            ($($t:ident)+) => {
                $(
                    {
                        assert_eq!(CheckedEuclid::checked_div_euclid(&$t::MIN, &-1), None);
                        assert_eq!(CheckedEuclid::checked_rem_euclid(&$t::MIN, &-1), None);
                        assert_eq!(CheckedEuclid::checked_div_euclid(&1, &0), None);
                        assert_eq!(CheckedEuclid::checked_rem_euclid(&1, &0), None);
                    }
                )+
            };
        }

        test_euclid_checked!(isize i8 i16 i32 i64 i128);
    }
}
