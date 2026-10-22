#[cfg(test)]
mod test {
    use super::*;
    // Upstream property generator unavailable in the maintained offline graph.
    #[cfg(any())]
    use proptest::prelude::*;

    #[track_caller]
    fn verify(input: &str, expected: Vec<(crate::Style, &str)>) {
        let expected = expected
            .into_iter()
            .map(|(style, value)| (style, value.to_owned()))
            .collect::<Vec<_>>();
        let mut state = WinconBytes::new();
        let actual = state.extract_next(input.as_bytes()).collect::<Vec<_>>();
        assert_eq!(expected, actual, "{input:?}");
    }

    #[test]
    fn start() {
        let green_on_red = crate::AnsiColor::Green.on(crate::AnsiColor::Red);
        let input = format!("{green_on_red}Hello{green_on_red:#} world!");
        let expected = vec![
            (green_on_red, "Hello"),
            (crate::Style::default(), " world!"),
        ];
        verify(&input, expected);
    }

    #[test]
    fn middle() {
        let green_on_red = crate::AnsiColor::Green.on(crate::AnsiColor::Red);
        let input = format!("Hello {green_on_red}world{green_on_red:#}!");
        let expected = vec![
            (crate::Style::default(), "Hello "),
            (green_on_red, "world"),
            (crate::Style::default(), "!"),
        ];
        verify(&input, expected);
    }

    #[test]
    fn end() {
        let green_on_red = crate::AnsiColor::Green.on(crate::AnsiColor::Red);
        let input = format!("Hello {green_on_red}world!{green_on_red:#}");
        let expected = vec![
            (crate::Style::default(), "Hello "),
            (green_on_red, "world!"),
        ];
        verify(&input, expected);
    }

    #[test]
    fn ansi256_colors() {
        let ansi_11 = crate::Ansi256Color(11).on_default();
        // termcolor only supports "brights" via these
        let input = format!("Hello {ansi_11}world{ansi_11:#}!");
        let expected = vec![
            (crate::Style::default(), "Hello "),
            (ansi_11, "world"),
            (crate::Style::default(), "!"),
        ];
        verify(&input, expected);
    }

    #[cfg(any())]
    proptest! {
        #[test]
        #[cfg_attr(miri, ignore)]  // See https://github.com/AltSysrq/proptest/issues/253
        fn wincon_no_escapes(s in "\\PC*") {
            let expected = if s.is_empty() {
                vec![]
            } else {
                vec![(crate::Style::default(), s.clone())]
            };
            let mut state = WinconBytes::new();
            let actual = state.extract_next(s.as_bytes()).collect::<Vec<_>>();
            assert_eq!(expected, actual);
        }
    }
}
