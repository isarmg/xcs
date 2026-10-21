#[cfg(test)]
mod tests {
    /// Test `CtNeg` impl on `i*`
    macro_rules! signed_ct_neg_tests {
         ( $($int:ident),+ ) => {
             $(
                mod $int {
                    use crate::{Choice, CtNeg};

                    #[test]
                    fn ct_neg() {
                        let n: $int = 42;
                        assert_eq!(n, n.ct_neg(Choice::FALSE));
                        assert_eq!(-n, n.ct_neg(Choice::TRUE));
                    }

                    #[test]
                    fn ct_neg_assign() {
                        let n: $int = 42;
                        let mut x = n;
                        x.ct_neg_assign(Choice::FALSE);
                        assert_eq!(n, x);

                        x.ct_neg_assign(Choice::TRUE);
                        assert_eq!(-n, x);
                    }
                }
             )+
        };
    }

    /// Test `CtNeg` impl on `u*`
    macro_rules! unsigned_ct_neg_tests {
         ( $($uint:ident),+ ) => {
             $(
                mod $uint {
                    use crate::{Choice, CtNeg};

                    #[test]
                    fn ct_neg() {
                        let n: $uint = 42;
                        assert_eq!(n, n.ct_neg(Choice::FALSE));
                        assert_eq!(<$uint>::MAX - n + 1, n.ct_neg(Choice::TRUE));
                    }

                    #[test]
                    fn ct_neg_assign() {
                        let n: $uint = 42;
                        let mut x = n;
                        x.ct_neg_assign(Choice::FALSE);
                        assert_eq!(n, x);

                        x.ct_neg_assign(Choice::TRUE);
                        assert_eq!(<$uint>::MAX - n + 1, x);
                    }
                }
             )+
        };
    }

    signed_ct_neg_tests!(i8, i16, i32, i64, i128, isize);
    unsigned_ct_neg_tests!(u8, u16, u32, u64, u128, usize);
}
