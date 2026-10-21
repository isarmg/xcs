#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod test {
    use super::*;

    #[test]
    fn keys() {
        let cases = [
            ("a", "a"),
            (r#""hello\n ""#, "hello\n "),
            (r"'hello\n '", "hello\\n "),
        ];

        for (input, expected) in cases {
            dbg!(input);
            let parsed = simple_key.parse(new_input(input));
            assert_eq!(
                parsed,
                Ok((RawString::with_span(0..(input.len())), expected.into())),
                "Parsing {input:?}"
            );
        }
    }
}
