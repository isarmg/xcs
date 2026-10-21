#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn one_digit() {
        let input = "verify\nthis";
        let expected = "    2: verify\n       this";
        let mut output = String::new();

        Indented {
            inner: &mut output,
            number: Some(2),
            started: false,
        }
        .write_str(input)
        .unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn two_digits() {
        let input = "verify\nthis";
        let expected = "   12: verify\n       this";
        let mut output = String::new();

        Indented {
            inner: &mut output,
            number: Some(12),
            started: false,
        }
        .write_str(input)
        .unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn no_digits() {
        let input = "verify\nthis";
        let expected = "    verify\n    this";
        let mut output = String::new();

        Indented {
            inner: &mut output,
            number: None,
            started: false,
        }
        .write_str(input)
        .unwrap();

        assert_eq!(expected, output);
    }
}
