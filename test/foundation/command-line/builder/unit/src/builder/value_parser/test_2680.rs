#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ensure_typed_applies_to_parse() {
        fn parse(_: &str) -> Result<usize, std::io::Error> {
            Ok(10)
        }
        let cmd = crate::Command::new("cmd");
        let arg = None;
        assert_eq!(
            TypedValueParser::parse_ref(&parse, &cmd, arg, std::ffi::OsStr::new("foo")).unwrap(),
            10
        );
    }
}
