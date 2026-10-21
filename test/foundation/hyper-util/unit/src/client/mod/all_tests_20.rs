#[cfg(all(test, any(feature = "client-legacy", feature = "client-proxy")))]
mod tests {
    use super::strip_ipv6_brackets;

    #[test]
    fn test_strip_ipv6_brackets() {
        assert_eq!(strip_ipv6_brackets("[::1]"), "::1");
        assert_eq!(strip_ipv6_brackets("::1"), "::1");
        assert_eq!(strip_ipv6_brackets("example.com"), "example.com");
        assert_eq!(strip_ipv6_brackets("[example.com"), "[example.com");
        assert_eq!(strip_ipv6_brackets("example.com]"), "example.com]");
    }
}
