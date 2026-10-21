#[test]
fn wrapping_from_str_radix() {
    macro_rules! test_wrapping_from_str_radix {
        ($($t:ty)+) => {
            $(
                for &(s, r) in &[("42", 10), ("42", 2), ("-13.0", 10), ("foo", 10)] {
                    let w = Wrapping::<$t>::from_str_radix(s, r).map(|w| w.0);
                    assert_eq!(w, <$t as Num>::from_str_radix(s, r));
                }
            )+
        };
    }

    test_wrapping_from_str_radix!(usize u8 u16 u32 u64 isize i8 i16 i32 i64);
}
