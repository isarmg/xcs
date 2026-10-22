#[cfg(test)]
#[cfg(feature = "std")]
mod test {
    use super::*;

    #[test]
    fn print_size_of() {
        use core::mem::size_of;
        dbg!(size_of::<Effects>());
        dbg!(size_of::<EffectsDisplay>());
    }

    #[test]
    fn no_align() {
        #[track_caller]
        fn assert_no_align(d: impl core::fmt::Display) {
            let expected = format!("{d}");
            let actual = format!("{d:<10}");
            assert_eq!(expected, actual);
        }

        assert_no_align(Effects::BOLD.render());
    }
}
