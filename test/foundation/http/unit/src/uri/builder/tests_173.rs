#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_from_str() {
        let uri = Builder::new()
            .scheme(Scheme::HTTP)
            .authority("hyper.rs")
            .path_and_query("/foo?a=1")
            .build()
            .unwrap();
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), "hyper.rs");
        assert_eq!(uri.path(), "/foo");
        assert_eq!(uri.query(), Some("a=1"));
    }

    #[test]
    fn build_from_string() {
        for i in 1..10 {
            let uri = Builder::new()
                .path_and_query(format!("/foo?a={}", i))
                .build()
                .unwrap();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
    }

    #[test]
    fn build_from_string_ref() {
        for i in 1..10 {
            let p_a_q = format!("/foo?a={}", i);
            let uri = Builder::new().path_and_query(&p_a_q).build().unwrap();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
    }

    #[test]
    fn build_from_empty_path_and_query() {
        let uri = Builder::new()
            .scheme(Scheme::HTTP)
            .authority("localhost:8080")
            .path_and_query("")
            .build()
            .unwrap();

        assert_eq!(uri, "http://localhost:8080");
        assert_eq!(uri.path(), "/");
    }

    #[test]
    fn empty_path_and_query_remains_strict() {
        assert!(PathAndQuery::try_from("").is_err());
    }

    #[test]
    fn authority_form_path_and_query_remains_strict() {
        assert!(Builder::new()
            .path_and_query("localhost:8080")
            .build()
            .is_err());
    }

    #[test]
    fn build_from_uri() {
        let original_uri = Uri::default();
        let uri = Builder::from(original_uri.clone()).build().unwrap();
        assert_eq!(original_uri, uri);
    }

    #[test]
    fn build_star_for_http2() {
        let uri = Builder::new()
            .scheme("https")
            .authority("example.com")
            .path_and_query("*")
            .build()
            .unwrap();

        assert_eq!(uri.scheme(), Some(&Scheme::HTTPS));
        assert_eq!(uri.host(), Some("example.com"));
        assert_eq!(uri.path(), "*");
    }
}
