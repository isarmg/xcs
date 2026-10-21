#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ext_traits::tests::{RequiresState, State},
        extract::FromRef,
    };
    use http::Method;

    #[tokio::test]
    async fn extract_without_state() {
        let req = Request::new(Body::empty());

        let method: Method = req.extract().await.unwrap();

        assert_eq!(method, Method::GET);
    }

    #[tokio::test]
    async fn extract_body_without_state() {
        let req = Request::new(Body::from("foobar"));

        let body: String = req.extract().await.unwrap();

        assert_eq!(body, "foobar");
    }

    #[tokio::test]
    async fn extract_with_state() {
        let req = Request::new(Body::empty());

        let state = "state".to_owned();

        let State(extracted_state): State<String> = req.extract_with_state(&state).await.unwrap();

        assert_eq!(extracted_state, state);
    }

    #[tokio::test]
    async fn extract_parts_without_state() {
        let mut req = Request::builder()
            .header("x-foo", "foo")
            .body(Body::empty())
            .unwrap();

        let method: Method = req.extract_parts().await.unwrap();

        assert_eq!(method, Method::GET);
        assert_eq!(req.headers()["x-foo"], "foo");
    }

    #[tokio::test]
    async fn extract_parts_with_state() {
        let mut req = Request::builder()
            .header("x-foo", "foo")
            .body(Body::empty())
            .unwrap();

        let state = "state".to_owned();

        let State(extracted_state): State<String> =
            req.extract_parts_with_state(&state).await.unwrap();

        assert_eq!(extracted_state, state);
        assert_eq!(req.headers()["x-foo"], "foo");
    }

    // this stuff just needs to compile
    #[allow(dead_code)]
    struct WorksForCustomExtractor {
        method: Method,
        from_state: String,
        body: String,
    }

    impl<S> FromRequest<S> for WorksForCustomExtractor
    where
        S: Send + Sync,
        String: FromRef<S> + FromRequest<()>,
    {
        type Rejection = <String as FromRequest<()>>::Rejection;

        async fn from_request(mut req: Request, state: &S) -> Result<Self, Self::Rejection> {
            let RequiresState(from_state) = req.extract_parts_with_state(state).await.unwrap();
            let method = req.extract_parts().await.unwrap();
            let body = req.extract().await?;

            Ok(Self {
                method,
                from_state,
                body,
            })
        }
    }
}
