#[test]
fn test_multinomial() {
    macro_rules! check_binomial {
        ($t:ty, $k:expr) => {{
            let n: $t = $k.iter().fold(0, |acc, &x| acc + x);
            let k: &[$t] = $k;
            assert_eq!(k.len(), 2);
            assert_eq!(multinomial(k), binomial(n, k[0]));
        }};
    }

    check_binomial!(u8, &[4, 5]);

    check_binomial!(i8, &[4, 5]);

    check_binomial!(u16, &[2, 98]);
    check_binomial!(u16, &[4, 10]);

    check_binomial!(i16, &[2, 98]);
    check_binomial!(i16, &[4, 10]);

    check_binomial!(u32, &[2, 98]);
    check_binomial!(u32, &[11, 24]);
    check_binomial!(u32, &[4, 10]);

    check_binomial!(i32, &[2, 98]);
    check_binomial!(i32, &[11, 24]);
    check_binomial!(i32, &[4, 10]);

    check_binomial!(u64, &[2, 98]);
    check_binomial!(u64, &[11, 24]);
    check_binomial!(u64, &[4, 10]);

    check_binomial!(i64, &[2, 98]);
    check_binomial!(i64, &[11, 24]);
    check_binomial!(i64, &[4, 10]);

    macro_rules! check_multinomial {
        ($t:ty, $k:expr, $r:expr) => {{
            let k: &[$t] = $k;
            let expected: $t = $r;
            assert_eq!(multinomial(k), expected);
        }};
    }

    check_multinomial!(u8, &[2, 1, 2], 30);
    check_multinomial!(u8, &[2, 3, 0], 10);

    check_multinomial!(i8, &[2, 1, 2], 30);
    check_multinomial!(i8, &[2, 3, 0], 10);

    check_multinomial!(u16, &[2, 1, 2], 30);
    check_multinomial!(u16, &[2, 3, 0], 10);

    check_multinomial!(i16, &[2, 1, 2], 30);
    check_multinomial!(i16, &[2, 3, 0], 10);

    check_multinomial!(u32, &[2, 1, 2], 30);
    check_multinomial!(u32, &[2, 3, 0], 10);

    check_multinomial!(i32, &[2, 1, 2], 30);
    check_multinomial!(i32, &[2, 3, 0], 10);

    check_multinomial!(u64, &[2, 1, 2], 30);
    check_multinomial!(u64, &[2, 3, 0], 10);

    check_multinomial!(i64, &[2, 1, 2], 30);
    check_multinomial!(i64, &[2, 3, 0], 10);

    check_multinomial!(u64, &[], 1);
    check_multinomial!(u64, &[0], 1);
    check_multinomial!(u64, &[12345], 1);
}
