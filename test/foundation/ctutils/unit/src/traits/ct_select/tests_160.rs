#[cfg(test)]
mod tests {
    use super::{Choice, CtSelect, cmp};

    macro_rules! ct_select_test_unsigned {
        ($ty:ty, $name:ident) => {
            #[test]
            fn $name() {
                let a: $ty = 1;
                let b: $ty = 2;
                assert_eq!(a.ct_select(&b, Choice::FALSE), a);
                assert_eq!(a.ct_select(&b, Choice::TRUE), b);
            }
        };
    }

    macro_rules! ct_select_test_signed {
        ($ty:ty, $name:ident) => {
            #[test]
            fn $name() {
                let a: $ty = 1;
                let b: $ty = -2;
                assert_eq!(a.ct_select(&b, Choice::FALSE), a);
                assert_eq!(a.ct_select(&b, Choice::TRUE), b);
            }
        };
    }

    ct_select_test_unsigned!(u8, u8_ct_select);
    ct_select_test_unsigned!(u16, u16_ct_select);
    ct_select_test_unsigned!(u32, u32_ct_select);
    ct_select_test_unsigned!(u64, u64_ct_select);
    ct_select_test_unsigned!(u128, u128_ct_select);
    ct_select_test_unsigned!(usize, usize_ct_select);

    ct_select_test_signed!(i8, i8_ct_select);
    ct_select_test_signed!(i16, i16_ct_select);
    ct_select_test_signed!(i32, i32_ct_select);
    ct_select_test_signed!(i64, i64_ct_select);
    ct_select_test_signed!(i128, i128_ct_select);
    ct_select_test_signed!(isize, isize_ct_select);

    #[test]
    fn ordering_ct_select() {
        let a = cmp::Ordering::Less;
        let b = cmp::Ordering::Greater;
        assert_eq!(a.ct_select(&b, Choice::FALSE), a);
        assert_eq!(a.ct_select(&b, Choice::TRUE), b);
    }
}
