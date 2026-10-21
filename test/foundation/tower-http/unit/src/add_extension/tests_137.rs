#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::test_helpers::Body;
    use http::Response;
    use std::{convert::Infallible, sync::Arc};
    use tower::{service_fn, ServiceBuilder, ServiceExt};

    struct State(i32);

    #[tokio::test]
    async fn basic() {
        let state = Arc::new(State(1));

        let svc = ServiceBuilder::new()
            .layer(AddExtensionLayer::new(state))
            .service(service_fn(|req: Request<Body>| async move {
                let state = req.extensions().get::<Arc<State>>().unwrap();
                Ok::<_, Infallible>(Response::new(state.0))
            }));

        let res = svc
            .oneshot(Request::new(Body::empty()))
            .await
            .unwrap()
            .into_body();

        assert_eq!(1, res);
    }
}
