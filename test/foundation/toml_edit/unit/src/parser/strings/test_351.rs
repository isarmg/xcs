#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod test {
    use super::*;

    #[test]
    fn basic_string() {
        let input =
            r#""I'm a string. \"You can quote me\". Name\tJos\u00E9\nLocation\tSF. \U0002070E""#;
        let expected = "I\'m a string. \"You can quote me\". Name\tJosé\nLocation\tSF. \u{2070E}";
        let parsed = string.parse(new_input(input));
        assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
    }

    #[test]
    fn ml_basic_string() {
        let cases = [
            (
                r#""""
Roses are red
Violets are blue""""#,
                r#"Roses are red
Violets are blue"#,
            ),
            (r#"""" \""" """"#, " \"\"\" "),
            (r#"""" \\""""#, " \\"),
        ];

        for &(input, expected) in &cases {
            let parsed = string.parse(new_input(input));
            assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
        }

        let invalid_cases = [r#""""  """#, r#""""  \""""#];

        for input in &invalid_cases {
            let parsed = string.parse(new_input(input));
            assert!(parsed.is_err());
        }
    }

    #[test]
    fn ml_basic_string_escape_ws() {
        let inputs = [
            r#""""
The quick brown \


  fox jumps over \
    the lazy dog.""""#,
            r#""""\
       The quick brown \
       fox jumps over \
       the lazy dog.\
       """"#,
        ];
        for input in &inputs {
            let expected = "The quick brown fox jumps over the lazy dog.";
            let parsed = string.parse(new_input(input));
            assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
        }
        let empties = [
            r#""""\
       """"#,
            r#""""
\
  \
""""#,
        ];
        for input in &empties {
            let expected = "";
            let parsed = string.parse(new_input(input));
            assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
        }
    }

    #[test]
    fn literal_string() {
        let inputs = [
            r"'C:\Users\nodejs\templates'",
            r"'\\ServerX\admin$\system32\'",
            r#"'Tom "Dubs" Preston-Werner'"#,
            r"'<\i\c*\s*>'",
        ];

        for input in &inputs {
            let expected = &input[1..input.len() - 1];
            let parsed = string.parse(new_input(input));
            assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
        }
    }

    #[test]
    fn ml_literal_string() {
        let inputs = [
            r"'''I [dw]on't need \d{2} apples'''",
            r#"''''one_quote''''"#,
        ];
        for input in &inputs {
            let expected = &input[3..input.len() - 3];
            let parsed = string.parse(new_input(input));
            assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
        }

        let input = r#"'''
The first newline is
trimmed in raw strings.
   All other whitespace
   is preserved.
'''"#;
        let expected = &input[4..input.len() - 3];
        let parsed = string.parse(new_input(input));
        assert_eq!(parsed.as_deref(), Ok(expected), "Parsing {input:?}");
    }
}
