#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_write_style_valid() {
        let inputs = vec![
            ("auto", WriteStyle::Auto),
            ("always", WriteStyle::Always),
            ("never", WriteStyle::Never),
        ];

        for (input, expected) in inputs {
            assert_eq!(expected, parse_write_style(input));
        }
    }

    #[test]
    fn parse_write_style_invalid() {
        let inputs = vec!["", "true", "false", "NEVER!!"];

        for input in inputs {
            assert_eq!(WriteStyle::Auto, parse_write_style(input));
        }
    }
}
