#[test]
fn test_binomial() {
    macro_rules! check {
        ($t:ty, $x:expr, $y:expr, $r:expr) => {{
            let x: $t = $x;
            let y: $t = $y;
            let expected: $t = $r;
            assert_eq!(binomial(x, y), expected);
            if y <= x {
                assert_eq!(binomial(x, x - y), expected);
            }
        }};
    }
    check!(u8, 9, 4, 126);
    check!(u8, 0, 0, 1);
    check!(u8, 2, 3, 0);

    check!(i8, 9, 4, 126);
    check!(i8, 0, 0, 1);
    check!(i8, 2, 3, 0);

    check!(u16, 100, 2, 4950);
    check!(u16, 14, 4, 1001);
    check!(u16, 0, 0, 1);
    check!(u16, 2, 3, 0);

    check!(i16, 100, 2, 4950);
    check!(i16, 14, 4, 1001);
    check!(i16, 0, 0, 1);
    check!(i16, 2, 3, 0);

    check!(u32, 100, 2, 4950);
    check!(u32, 35, 11, 417225900);
    check!(u32, 14, 4, 1001);
    check!(u32, 0, 0, 1);
    check!(u32, 2, 3, 0);

    check!(i32, 100, 2, 4950);
    check!(i32, 35, 11, 417225900);
    check!(i32, 14, 4, 1001);
    check!(i32, 0, 0, 1);
    check!(i32, 2, 3, 0);

    check!(u64, 100, 2, 4950);
    check!(u64, 35, 11, 417225900);
    check!(u64, 14, 4, 1001);
    check!(u64, 0, 0, 1);
    check!(u64, 2, 3, 0);

    check!(i64, 100, 2, 4950);
    check!(i64, 35, 11, 417225900);
    check!(i64, 14, 4, 1001);
    check!(i64, 0, 0, 1);
    check!(i64, 2, 3, 0);
}
