#[cfg(test)]
mod tests {
    #![allow(unreachable_code)]

    use super::*;
    use crate::test_helpers::Body;
    use http::Response;
    use std::convert::Infallible;
    use service_stack::{ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn panic_before_returning_future() {
        let svc = ServiceBuilder::new()
            .layer(CatchPanicLayer::new())
            .service_fn(|_: Request<Body>| {
                panic!("service panic");
                async { Ok::<_, Infallible>(Response::new(Body::empty())) }
            });

        let req = Request::new(Body::empty());

        let res = svc.oneshot(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = crate::test_helpers::to_bytes(res).await.unwrap();
        assert_eq!(&body[..], b"Service panicked");
    }

    #[tokio::test]
    async fn panic_in_future() {
        let svc = ServiceBuilder::new()
            .layer(CatchPanicLayer::new())
            .service_fn(|_: Request<Body>| async {
                panic!("future panic");
                Ok::<_, Infallible>(Response::new(Body::empty()))
            });

        let req = Request::new(Body::empty());

        let res = svc.oneshot(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = crate::test_helpers::to_bytes(res).await.unwrap();
        assert_eq!(&body[..], b"Service panicked");
    }
}
