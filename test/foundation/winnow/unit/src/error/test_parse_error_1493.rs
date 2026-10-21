#[cfg(test)]
#[cfg(feature = "std")]
mod test_parse_error {
    use super::*;

    #[test]
    fn single_line() {
        let mut input = "0xZ123";
        let start = input.checkpoint();
        let _ = input.next_token().unwrap();
        let _ = input.next_token().unwrap();
        let inner = InputError::at(input);
        let error = ParseError::new(input, start, inner);
        let expected = "\
0xZ123
  ^
failed to parse starting at: Z123";
        assert_eq!(error.to_string(), expected);
    }
}
