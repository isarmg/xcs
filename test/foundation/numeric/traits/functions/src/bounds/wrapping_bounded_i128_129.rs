#[test]
fn wrapping_bounded_i128() {
    macro_rules! test_wrapping_bounded {
        ($($t:ty)+) => {
            $(
                assert_eq!(<Wrapping<$t> as Bounded>::min_value().0, <$t>::MIN);
                assert_eq!(<Wrapping<$t> as Bounded>::max_value().0, <$t>::MAX);
            )+
        };
    }

    test_wrapping_bounded!(u128 i128);
}
