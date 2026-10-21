#[cfg(test)]
mod tests {
    use crate::{
        routing::{on, MethodFilter},
        test_helpers::TestClient,
        Router,
    };

    use super::*;
    use axum_core::body::Body;
    use http::{Method, Request};
    use mime::APPLICATION_WWW_FORM_URLENCODED;
    use serde::{Deserialize, Serialize};
    use std::fmt::Debug;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Pagination {
        size: Option<u64>,
        page: Option<u64>,
    }

    async fn check_query<T: DeserializeOwned + PartialEq + Debug>(uri: impl AsRef<str>, value: T) {
        let req = Request::builder()
            .uri(uri.as_ref())
            .body(Body::empty())
            .unwrap();
        assert_eq!(Form::<T>::from_request(req, &()).await.unwrap().0, value);
    }

    async fn check_body<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: T) {
        let req = Request::builder()
            .uri("http://example.com/test")
            .method(Method::POST)
            .header(CONTENT_TYPE, APPLICATION_WWW_FORM_URLENCODED.as_ref())
            .body(Body::from(serde_urlencoded::to_string(&value).unwrap()))
            .unwrap();
        assert_eq!(Form::<T>::from_request(req, &()).await.unwrap().0, value);
    }

    #[crate::test]
    async fn test_form_query() {
        check_query(
            "http://example.com/test",
            Pagination {
                size: None,
                page: None,
            },
        )
        .await;

        check_query(
            "http://example.com/test?size=10",
            Pagination {
                size: Some(10),
                page: None,
            },
        )
        .await;

        check_query(
            "http://example.com/test?size=10&page=20",
            Pagination {
                size: Some(10),
                page: Some(20),
            },
        )
        .await;
    }

    #[crate::test]
    async fn test_form_body() {
        check_body(Pagination {
            size: None,
            page: None,
        })
        .await;

        check_body(Pagination {
            size: Some(10),
            page: None,
        })
        .await;

        check_body(Pagination {
            size: Some(10),
            page: Some(20),
        })
        .await;
    }

    #[crate::test]
    async fn test_incorrect_content_type() {
        let req = Request::builder()
            .uri("http://example.com/test")
            .method(Method::POST)
            .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(
                serde_urlencoded::to_string(&Pagination {
                    size: Some(10),
                    page: None,
                })
                .unwrap(),
            ))
            .unwrap();
        assert!(matches!(
            Form::<Pagination>::from_request(req, &())
                .await
                .unwrap_err(),
            FormRejection::InvalidFormContentType(InvalidFormContentType)
        ));
    }

    #[tokio::test]
    async fn deserialize_error_status_codes() {
        #[allow(dead_code)]
        #[derive(Deserialize)]
        struct Payload {
            a: i32,
        }

        let app = Router::new().route(
            "/",
            on(
                MethodFilter::GET.or(MethodFilter::POST),
                |_: Form<Payload>| async {},
            ),
        );

        let client = TestClient::new(app);

        let res = client.get("/?a=false").await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Failed to deserialize form: a: invalid digit found in string"
        );

        let res = client
            .post("/")
            .header(CONTENT_TYPE, APPLICATION_WWW_FORM_URLENCODED.as_ref())
            .body("a=false")
            .await;
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            res.text().await,
            "Failed to deserialize form body: a: invalid digit found in string"
        );
    }
}
