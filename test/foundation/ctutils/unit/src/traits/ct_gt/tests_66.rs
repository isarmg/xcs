#[cfg(test)]
mod tests {
    use super::CtGt;
    use core::cmp::Ordering;

    /// Test `CtGt`
    macro_rules! ct_gt_tests {
         ( $($int:ident),+ ) => {
             $(
                mod $int {
                    use super::CtGt;

                    #[test]
                    fn ct_gt() {
                        let a = <$int>::MIN;
                        let b = <$int>::MAX;
                        assert!(!a.ct_gt(&a).to_bool());
                        assert!(!a.ct_gt(&b).to_bool());
                        assert!(b.ct_gt(&a).to_bool());
                    }

                }
             )+
        };
    }

    ct_gt_tests!(u8, u16, u32, u64, u128, usize);

    #[test]
    fn ordering() {
        assert!(!Ordering::Equal.ct_gt(&Ordering::Equal).to_bool());
        assert!(!Ordering::Less.ct_gt(&Ordering::Greater).to_bool());
        assert!(Ordering::Greater.ct_gt(&Ordering::Less).to_bool());
    }
}
