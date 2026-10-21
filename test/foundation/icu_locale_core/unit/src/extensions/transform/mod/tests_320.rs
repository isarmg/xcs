#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_extension_fromstr() {
        let te: Transform = "t-en-us-h0-hybrid"
            .parse()
            .expect("Failed to parse Transform");
        assert_eq!(te.to_string(), "t-en-us-h0-hybrid");

        let te: Result<Transform, _> = "t".parse();
        assert!(te.is_err());
    }
}
