#[cfg(test)]
#[cfg(feature = "std")]
mod test {
    use super::*;

    #[test]
    fn max_display_buffer() {
        let c = RgbColor(255, 255, 255);
        let actual = c.render_fg().to_string();
        assert_eq!(actual, "\u{1b}[38;2;255;255;255m");
        assert_eq!(actual.len(), DISPLAY_BUFFER_CAPACITY);
    }

    #[test]
    fn print_size_of() {
        use core::mem::size_of;
        dbg!(size_of::<Color>());
        dbg!(size_of::<AnsiColor>());
        dbg!(size_of::<Ansi256Color>());
        dbg!(size_of::<RgbColor>());
        dbg!(size_of::<DisplayBuffer>());
    }

    #[test]
    fn no_align() {
        #[track_caller]
        fn assert_no_align(d: impl core::fmt::Display) {
            let expected = format!("{d}");
            let actual = format!("{d:<10}");
            assert_eq!(expected, actual);
        }

        assert_no_align(AnsiColor::White.render_fg());
        assert_no_align(AnsiColor::White.render_bg());
        assert_no_align(Ansi256Color(0).render_fg());
        assert_no_align(Ansi256Color(0).render_bg());
        assert_no_align(RgbColor(0, 0, 0).render_fg());
        assert_no_align(RgbColor(0, 0, 0).render_bg());
        assert_no_align(Color::Ansi(AnsiColor::White).render_fg());
        assert_no_align(Color::Ansi(AnsiColor::White).render_bg());
    }
}
