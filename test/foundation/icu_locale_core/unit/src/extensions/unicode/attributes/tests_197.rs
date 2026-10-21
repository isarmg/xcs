#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attributes_fromstr() {
        let attrs: Attributes = "foo-bar".parse().expect("Failed to parse Attributes");
        assert_eq!(attrs.to_string(), "bar-foo");
    }
}
