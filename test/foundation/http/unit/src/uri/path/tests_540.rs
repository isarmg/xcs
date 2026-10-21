#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_to_self_of_same_path() {
        let p1: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        let p2: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_eq!(p1, p2);
        assert_eq!(p2, p1);
    }

    #[test]
    fn not_equal_to_self_of_different_path() {
        let p1: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        let p2: PathAndQuery = "/world&foo=bar".parse().unwrap();
        assert_ne!(p1, p2);
        assert_ne!(p2, p1);
    }

    #[test]
    fn equates_with_a_str() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_eq!(&path_and_query, "/hello/world&foo=bar");
        assert_eq!("/hello/world&foo=bar", &path_and_query);
        assert_eq!(path_and_query, "/hello/world&foo=bar");
        assert_eq!("/hello/world&foo=bar", path_and_query);
    }

    #[test]
    fn not_equal_with_a_str_of_a_different_path() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        // as a reference
        assert_ne!(&path_and_query, "/hello&foo=bar");
        assert_ne!("/hello&foo=bar", &path_and_query);
        // without reference
        assert_ne!(path_and_query, "/hello&foo=bar");
        assert_ne!("/hello&foo=bar", path_and_query);
    }

    #[test]
    fn equates_with_a_string() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_eq!(path_and_query, "/hello/world&foo=bar".to_string());
        assert_eq!("/hello/world&foo=bar".to_string(), path_and_query);
    }

    #[test]
    fn not_equal_with_a_string_of_a_different_path() {
        let path_and_query: PathAndQuery = "/hello/world&foo=bar".parse().unwrap();
        assert_ne!(path_and_query, "/hello&foo=bar".to_string());
        assert_ne!("/hello&foo=bar".to_string(), path_and_query);
    }

    #[test]
    fn compares_to_self() {
        let p1: PathAndQuery = "/a/world&foo=bar".parse().unwrap();
        let p2: PathAndQuery = "/b/world&foo=bar".parse().unwrap();
        assert!(p1 < p2);
        assert!(p2 > p1);
    }

    #[test]
    fn compares_with_a_str() {
        let path_and_query: PathAndQuery = "/b/world&foo=bar".parse().unwrap();
        // by ref
        assert!(&path_and_query < "/c/world&foo=bar");
        assert!("/c/world&foo=bar" > &path_and_query);
        assert!(&path_and_query > "/a/world&foo=bar");
        assert!("/a/world&foo=bar" < &path_and_query);

        // by val
        assert!(path_and_query < "/c/world&foo=bar");
        assert!("/c/world&foo=bar" > path_and_query);
        assert!(path_and_query > "/a/world&foo=bar");
        assert!("/a/world&foo=bar" < path_and_query);
    }

    #[test]
    fn compares_with_a_string() {
        let path_and_query: PathAndQuery = "/b/world&foo=bar".parse().unwrap();
        assert!(path_and_query < "/c/world&foo=bar".to_string());
        assert!("/c/world&foo=bar".to_string() > path_and_query);
        assert!(path_and_query > "/a/world&foo=bar".to_string());
        assert!("/a/world&foo=bar".to_string() < path_and_query);
    }

    #[test]
    fn ignores_valid_percent_encodings() {
        assert_eq!("/a%20b", pq("/a%20b?r=1").path());
        assert_eq!("qr=%31", pq("/a/b?qr=%31").query().unwrap());
    }

    #[test]
    fn ignores_invalid_percent_encodings() {
        assert_eq!("/a%%b", pq("/a%%b?r=1").path());
        assert_eq!("/aaa%", pq("/aaa%").path());
        assert_eq!("/aaa%", pq("/aaa%?r=1").path());
        assert_eq!("/aa%2", pq("/aa%2").path());
        assert_eq!("/aa%2", pq("/aa%2?r=1").path());
        assert_eq!("qr=%3", pq("/a/b?qr=%3").query().unwrap());
    }

    #[test]
    fn allow_utf8_in_path() {
        assert_eq!("/🍕", pq("/🍕").path());
    }

    #[test]
    fn allow_utf8_in_query() {
        assert_eq!(Some("pizza=🍕"), pq("/test?pizza=🍕").query());
    }

    #[test]
    fn rejects_invalid_utf8_in_path() {
        PathAndQuery::try_from(&[b'/', 0xFF][..]).expect_err("reject invalid utf8");
    }

    #[test]
    fn rejects_invalid_utf8_in_query() {
        PathAndQuery::try_from(&[b'/', b'a', b'?', 0xFF][..]).expect_err("reject invalid utf8");
    }

    #[test]
    fn rejects_empty_string() {
        PathAndQuery::try_from("").expect_err("reject empty str");
    }

    #[test]
    fn requires_starting_with_slash() {
        PathAndQuery::try_from("sneaky").expect_err("reject missing slash");
    }

    #[test]
    fn rejects_del_in_path() {
        PathAndQuery::try_from(&[b'/', 0x7F][..]).expect_err("reject DEL");
    }

    #[test]
    fn rejects_del_in_query() {
        PathAndQuery::try_from(&[b'/', b'a', b'?', 0x7F][..]).expect_err("reject DEL");
    }

    #[test]
    fn rejects_too_long_path_and_query() {
        let path = format!("/{}?query", "a".repeat(MAX_LEN));
        let err = PathAndQuery::try_from(path).expect_err("reject overly long path and query");
        assert_eq!(err.0, ErrorKind::TooLong);
    }

    #[test]
    fn accepts_max_length_path_and_query() {
        let path = format!("/{}?", "a".repeat(MAX_LEN - 2));
        let path_and_query = PathAndQuery::try_from(path).expect("accept maximum length");
        assert_eq!(path_and_query.as_str().len(), MAX_LEN);
        assert_eq!(path_and_query.query(), Some(""));
    }

    #[test]
    fn json_is_fine() {
        assert_eq!(
            r#"/{"bread":"baguette"}"#,
            pq(r#"/{"bread":"baguette"}"#).path()
        );
    }

    fn pq(s: &str) -> PathAndQuery {
        s.parse().expect(&format!("parsing {}", s))
    }
}
