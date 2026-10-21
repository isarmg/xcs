#[cfg(test)]
mod tests {
    use super::CtEq;
    use core::cmp::Ordering;

    macro_rules! truth_table {
        ($a:expr, $b:expr, $c:expr) => {
            assert!($a.ct_eq(&$b).to_bool());
            assert!(!$a.ct_eq(&$c).to_bool());
            assert!(!$b.ct_eq(&$c).to_bool());

            assert!(!$a.ct_ne(&$b).to_bool());
            assert!($a.ct_ne(&$c).to_bool());
            assert!($b.ct_ne(&$c).to_bool());
        };
    }

    macro_rules! ct_eq_test_unsigned {
        ($ty:ty, $name:ident) => {
            #[test]
            fn $name() {
                let a = <$ty>::MAX;
                let b = <$ty>::MAX;
                let c = <$ty>::MIN;
                truth_table!(a, b, c);
            }
        };
    }

    macro_rules! ct_eq_test_signed {
        ($ty:ty, $name:ident) => {
            #[test]
            fn $name() {
                let a = <$ty>::MAX;
                let b = <$ty>::MAX;
                let c = <$ty>::MIN;
                truth_table!(a, b, c);
            }
        };
    }

    ct_eq_test_unsigned!(u8, u8_ct_eq);
    ct_eq_test_unsigned!(u16, u16_ct_eq);
    ct_eq_test_unsigned!(u32, u32_ct_eq);
    ct_eq_test_unsigned!(u64, u64_ct_eq);
    ct_eq_test_unsigned!(u128, u128_ct_eq);
    ct_eq_test_unsigned!(usize, usize_ct_eq);

    ct_eq_test_signed!(i8, i8_ct_eq);
    ct_eq_test_signed!(i16, i16_ct_eq);
    ct_eq_test_signed!(i32, i32_ct_eq);
    ct_eq_test_signed!(i64, i64_ct_eq);
    ct_eq_test_signed!(i128, i128_ct_eq);
    ct_eq_test_signed!(isize, isize_ct_eq);

    #[test]
    fn array_ct_eq() {
        let a = [1u64, 2, 3];
        let b = [1u64, 2, 3];
        let c = [1u64, 2, 4];
        truth_table!(a, b, c);
    }

    #[test]
    fn ordering_ct_eq() {
        let a = Ordering::Greater;
        let b = Ordering::Greater;
        let c = Ordering::Less;
        truth_table!(a, b, c);
    }

    #[test]
    fn slice_ct_eq() {
        let a: &[u64] = &[1, 2, 3];
        let b: &[u64] = &[1, 2, 3];
        let c: &[u64] = &[1, 2, 4];
        truth_table!(a, b, c);

        // Length mismatches
        assert!(a.ct_ne(&[]).to_bool());
        assert!(a.ct_ne(&[1, 2]).to_bool());
    }
}
