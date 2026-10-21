#[cfg(all(test, feature = "http1"))]
mod tests {
    use super::te_is_trailers;
    use http::header::{HeaderValue, TE};
    use http::HeaderMap;

    #[test]
    fn te_is_trailers_accepts_comma_separated_values() {
        let mut headers = HeaderMap::new();
        headers.insert(TE, HeaderValue::from_static("gzip, Trailers"));

        assert!(te_is_trailers(&headers));
    }

    #[test]
    fn te_is_trailers_accepts_multiple_header_lines() {
        let mut headers = HeaderMap::new();
        headers.append(TE, HeaderValue::from_static("gzip"));
        headers.append(TE, HeaderValue::from_static("trailers"));

        assert!(te_is_trailers(&headers));
    }

    #[test]
    fn te_is_trailers_rejects_missing_trailers_token() {
        let mut headers = HeaderMap::new();
        headers.insert(TE, HeaderValue::from_static("gzip"));

        assert!(!te_is_trailers(&headers));
    }
}
