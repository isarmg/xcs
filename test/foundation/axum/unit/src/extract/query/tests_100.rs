#[cfg(test)]
mod tests {
    use crate::{routing::get, test_helpers::TestClient, Router};

    use super::*;
    use axum_core::{body::Body, extract::FromRequest};
    use http::{Request, StatusCode};
    use data_serialization::Deserialize;
    use std::fmt::Debug;

    async fn check<T>(uri: impl AsRef<str>, value: T)
    where
        T: DeserializeOwned + PartialEq + Debug,
    {
        let req = Request::builder()
            .uri(uri.as_ref())
            .body(Body::empty())
            .unwrap();
        assert_eq!(Query::<T>::from_request(req, &()).await.unwrap().0, value);
    }

    #[crate::test]
    async fn test_query() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Pagination {
            size: Option<u64>,
            page: Option<u64>,
        }

        check(
            "http://example.com/test",
            Pagination {
                size: None,
                page: None,
            },
        )
        .await;

        check(
            "http://example.com/test?size=10",
            Pagination {
                size: Some(10),
                page: None,
            },
        )
        .await;

        check(
            "http://example.com/test?size=10&page=20",
            Pagination {
                size: Some(10),
                page: Some(20),
            },
        )
        .await;
    }

    #[crate::test]
    async fn correct_rejection_status_code() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Params {
            n: i32,
        }

        async fn handler(_: Query<Params>) {}

        let app = Router::new().route("/", get(handler));
        let client = TestClient::new(app);

        let res = client.get("/?n=hi").await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Failed to deserialize query string: n: invalid digit found in string"
        );
    }

    #[test]
    fn test_try_from_uri() {
        #[derive(Deserialize)]
        struct TestQueryParams {
            foo: String,
            bar: u32,
        }
        let uri: Uri = "http://example.com/path?foo=hello&bar=42".parse().unwrap();
        let result: Query<TestQueryParams> = Query::try_from_uri(&uri).unwrap();
        assert_eq!(result.foo, String::from("hello"));
        assert_eq!(result.bar, 42);
    }

    #[test]
    fn test_try_from_uri_with_invalid_query() {
        #[derive(Deserialize)]
        struct TestQueryParams {
            _foo: String,
            _bar: u32,
        }
        let uri: Uri = "http://example.com/path?foo=hello&bar=invalid"
            .parse()
            .unwrap();
        let result: Result<Query<TestQueryParams>, _> = Query::try_from_uri(&uri);

        assert!(result.is_err());
    }
}
