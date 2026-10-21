#[cfg(test)]
mod test {
    #[cfg(feature = "getrandom")]
    #[test]
    fn test_error_codes() {
        // Make sure the values are the same as in `getrandom`.
        assert_eq!(super::Error::CUSTOM_START, getrandom::Error::CUSTOM_START);
        assert_eq!(super::Error::INTERNAL_START, getrandom::Error::INTERNAL_START);
    }
}
