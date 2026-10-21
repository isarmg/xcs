#[test]
fn test_iter_binomial() {
    macro_rules! check_simple {
        ($t:ty) => {{
            let n: $t = 3;
            let expected = [1, 3, 3, 1];
            for (b, &e) in IterBinomial::new(n).zip(&expected) {
                assert_eq!(b, e);
            }
        }};
    }

    check_simple!(u8);
    check_simple!(i8);
    check_simple!(u16);
    check_simple!(i16);
    check_simple!(u32);
    check_simple!(i32);
    check_simple!(u64);
    check_simple!(i64);

    macro_rules! check_binomial {
        ($t:ty, $n:expr) => {{
            let n: $t = $n;
            let mut k: $t = 0;
            for b in IterBinomial::new(n) {
                assert_eq!(b, binomial(n, k));
                k += 1;
            }
        }};
    }

    // Check the largest n for which there is no overflow.
    check_binomial!(u8, 10);
    check_binomial!(i8, 9);
    check_binomial!(u16, 18);
    check_binomial!(i16, 17);
    check_binomial!(u32, 34);
    check_binomial!(i32, 33);
    check_binomial!(u64, 67);
    check_binomial!(i64, 66);
}
