#[test]
fn test_lcm_overflow() {
    macro_rules! check {
        ($t:ty, $x:expr, $y:expr, $r:expr) => {{
            let x: $t = $x;
            let y: $t = $y;
            let o = x.checked_mul(y);
            assert!(
                o.is_none(),
                "sanity checking that {} input {} * {} overflows",
                stringify!($t),
                x,
                y
            );
            assert_eq!(x.lcm(&y), $r);
            assert_eq!(y.lcm(&x), $r);
        }};
    }

    // Original bug (Issue #166)
    check!(i64, 46656000000000000, 600, 46656000000000000);

    check!(i8, 0x40, 0x04, 0x40);
    check!(u8, 0x80, 0x02, 0x80);
    check!(i16, 0x40_00, 0x04, 0x40_00);
    check!(u16, 0x80_00, 0x02, 0x80_00);
    check!(i32, 0x4000_0000, 0x04, 0x4000_0000);
    check!(u32, 0x8000_0000, 0x02, 0x8000_0000);
    check!(i64, 0x4000_0000_0000_0000, 0x04, 0x4000_0000_0000_0000);
    check!(u64, 0x8000_0000_0000_0000, 0x02, 0x8000_0000_0000_0000);
}
