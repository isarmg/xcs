#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_other_extension_fromstr() {
        let oe: Other = "o-foo-bar".parse().expect("Failed to parse Other");
        assert_eq!(oe.to_string(), "o-foo-bar");

        let oe: Other = "1-foo-bar".parse().expect("Failed to parse Other");
        assert_eq!(oe.to_string(), "1-foo-bar");

        let oe: Result<Other, _> = "o".parse();
        assert!(oe.is_err());
    }
}
