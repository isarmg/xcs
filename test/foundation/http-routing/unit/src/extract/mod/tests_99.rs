#[cfg(test)]
mod tests {
    use crate::{routing::get, test_helpers::*, Router};

    #[crate::test]
    async fn consume_body() {
        let app = Router::new().route("/", get(|body: String| async { body }));

        let client = TestClient::new(app);
        let res = client.get("/").body("foo").await;
        let body = res.text().await;

        assert_eq!(body, "foo");
    }
}
