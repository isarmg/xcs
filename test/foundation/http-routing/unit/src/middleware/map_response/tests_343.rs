#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::{test_helpers::TestClient, Router};

    #[crate::test]
    async fn works() {
        async fn add_header<B>(mut res: Response<B>) -> Response<B> {
            res.headers_mut().insert("x-foo", "foo".parse().unwrap());
            res
        }

        let app = Router::new().layer(map_response(add_header));
        let client = TestClient::new(app);

        let res = client.get("/").await;

        assert_eq!(res.headers()["x-foo"], "foo");
    }
}
