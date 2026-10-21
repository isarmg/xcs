#[test]
fn wrapping_bounded() {
    macro_rules! test_wrapping_bounded {
        ($($t:ty)+) => {
            $(
                assert_eq!(<Wrapping<$t> as Bounded>::min_value().0, <$t>::MIN);
                assert_eq!(<Wrapping<$t> as Bounded>::max_value().0, <$t>::MAX);
            )+
        };
    }

    test_wrapping_bounded!(usize u8 u16 u32 u64 isize i8 i16 i32 i64);
}
