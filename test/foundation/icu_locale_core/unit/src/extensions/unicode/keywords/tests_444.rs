#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords_fromstr() {
        let kw: Keywords = "hc-h12".parse().expect("Failed to parse Keywords");
        assert_eq!(kw.to_string(), "hc-h12");
    }
}
