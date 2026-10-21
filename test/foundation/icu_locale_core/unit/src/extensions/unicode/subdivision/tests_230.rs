#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subdivisionid_fromstr() {
        let si: SubdivisionId = "gbzzzz".parse().expect("Failed to parse SubdivisionId");
        assert_eq!(si.region.to_string(), "GB");
        assert_eq!(si.suffix.to_string(), "zzzz");
        assert_eq!(si.to_string(), "gbzzzz");

        for sample in ["", "gb", "o"] {
            let oe: Result<SubdivisionId, _> = sample.parse();
            assert!(oe.is_err(), "Should fail: {sample}");
        }
    }
}
