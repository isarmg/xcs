#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn into_url_file_scheme() {
        let err = "file:///etc/hosts".into_url().unwrap_err();
        assert_eq!(
            err.source().unwrap().to_string(),
            "URL scheme is not allowed"
        );
    }

    #[test]
    fn into_url_blob_scheme() {
        let err = "blob:https://example.com".into_url().unwrap_err();
        assert_eq!(
            err.source().unwrap().to_string(),
            "URL scheme is not allowed"
        );
    }


}
