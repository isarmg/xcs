#[cfg(test)]
mod tests {
    use super::{ResponseBuilderExt, ResponseUrl};
    use http::response::Builder;
    use url::Url;

    #[test]
    fn test_response_builder_ext() {
        let url = Url::parse("http://example.com").unwrap();
        let response = Builder::new()
            .status(200)
            .url(url.clone())
            .body(())
            .unwrap();

        assert_eq!(
            response.extensions().get::<ResponseUrl>(),
            Some(&ResponseUrl(url))
        );
    }
}
