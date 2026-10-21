#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_extension_fromstr() {
        let pe: Private = "x-foo-bar-l-baz".parse().expect("Failed to parse Private");
        assert_eq!(pe.to_string(), "x-foo-bar-l-baz");

        let pe: Result<Private, _> = "x".parse();
        assert!(pe.is_err());
    }
}
