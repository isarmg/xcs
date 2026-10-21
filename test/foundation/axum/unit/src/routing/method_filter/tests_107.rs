#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_http_method() {
        assert_eq!(
            MethodFilter::try_from(Method::CONNECT).unwrap(),
            MethodFilter::CONNECT
        );

        assert_eq!(
            MethodFilter::try_from(Method::DELETE).unwrap(),
            MethodFilter::DELETE
        );

        assert_eq!(
            MethodFilter::try_from(Method::GET).unwrap(),
            MethodFilter::GET
        );

        assert_eq!(
            MethodFilter::try_from(Method::HEAD).unwrap(),
            MethodFilter::HEAD
        );

        assert_eq!(
            MethodFilter::try_from(Method::OPTIONS).unwrap(),
            MethodFilter::OPTIONS
        );

        assert_eq!(
            MethodFilter::try_from(Method::PATCH).unwrap(),
            MethodFilter::PATCH
        );

        assert_eq!(
            MethodFilter::try_from(Method::POST).unwrap(),
            MethodFilter::POST
        );

        assert_eq!(
            MethodFilter::try_from(Method::PUT).unwrap(),
            MethodFilter::PUT
        );

        assert_eq!(
            MethodFilter::try_from(Method::TRACE).unwrap(),
            MethodFilter::TRACE
        );

        assert!(
            MethodFilter::try_from(http::Method::from_bytes(b"CUSTOM").unwrap())
                .unwrap_err()
                .to_string()
                .contains("CUSTOM")
        );
    }
}
