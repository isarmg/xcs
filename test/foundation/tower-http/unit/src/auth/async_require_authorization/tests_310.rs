#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::test_helpers::Body;
    use futures::future::BoxFuture;
    use http::{header, StatusCode};
    use service_stack::{BoxError, ServiceBuilder, ServiceExt};

    #[derive(Clone, Copy)]
    struct MyAuth;

    impl<B> AsyncAuthorizeRequest<B> for MyAuth
    where
        B: Send + 'static,
    {
        type RequestBody = B;
        type ResponseBody = Body;
        type Future = BoxFuture<'static, Result<Request<B>, Response<Self::ResponseBody>>>;

        fn authorize(&mut self, request: Request<B>) -> Self::Future {
            Box::pin(async move {
                let authorized = request
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|auth| auth.to_str().ok()?.strip_prefix("Bearer "))
                    == Some("69420");

                if authorized {
                    Ok(request)
                } else {
                    Err(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::empty())
                        .unwrap())
                }
            })
        }
    }

    #[tokio::test]
    async fn require_async_auth_works() {
        let mut service = ServiceBuilder::new()
            .layer(AsyncRequireAuthorizationLayer::new(MyAuth))
            .service_fn(echo);

        let request = Request::get("/")
            .header(header::AUTHORIZATION, "Bearer 69420")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn require_async_auth_401() {
        let mut service = ServiceBuilder::new()
            .layer(AsyncRequireAuthorizationLayer::new(MyAuth))
            .service_fn(echo);

        let request = Request::get("/")
            .header(header::AUTHORIZATION, "Bearer deez")
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }
}
