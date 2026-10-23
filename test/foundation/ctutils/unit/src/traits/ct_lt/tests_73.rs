#[cfg(test)]
mod tests {
    use super::CtLt;
    use core::cmp::Ordering;

    #[test]
    fn ct_lt() {
        let a = 42u64;
        let b = 43u64;
        assert!(!a.ct_lt(&a).to_bool());
        assert!(a.ct_lt(&b).to_bool());
        assert!(!b.ct_lt(&a).to_bool());
    }

    /// Test `CtLt`
    macro_rules! ct_lt_tests {
         ( $($int:ident),+ ) => {
             $(
                mod $int {
                    use super::CtLt;

                    #[test]
                    fn ct_gt() {
                        let a = <$int>::MIN;
                        let b = <$int>::MAX;
                        assert!(!a.ct_lt(&a).to_bool());
                        assert!(a.ct_lt(&b).to_bool());
                        assert!(!b.ct_lt(&a).to_bool());
                    }

                }
             )+
        };
    }

    ct_lt_tests!(u8, u16, u32, u64, u128, usize);

    #[test]
    fn ordering() {
        assert!(!Ordering::Equal.ct_lt(&Ordering::Equal).to_bool());
        assert!(Ordering::Less.ct_lt(&Ordering::Greater).to_bool());
        assert!(!Ordering::Greater.ct_lt(&Ordering::Less).to_bool());
    }
}
