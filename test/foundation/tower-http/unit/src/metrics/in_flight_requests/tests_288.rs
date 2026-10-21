#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::test_helpers::Body;
    use http::Request;
    use service_stack::{BoxError, ServiceBuilder};

    #[tokio::test]
    async fn basic() {
        let (in_flight_requests_layer, counter) = InFlightRequestsLayer::pair();

        let mut service = ServiceBuilder::new()
            .layer(in_flight_requests_layer)
            .service_fn(echo);
        assert_eq!(counter.get(), 0);

        // driving service to ready shouldn't increment the counter
        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        assert_eq!(counter.get(), 0);

        // creating the response future should increment the count
        let response_future = service.call(Request::new(Body::empty()));
        assert_eq!(counter.get(), 1);

        // count shouldn't decrement until the full body has been comsumed
        let response = response_future.await.unwrap();
        assert_eq!(counter.get(), 1);

        let body = response.into_body();
        crate::test_helpers::to_bytes(body).await.unwrap();
        assert_eq!(counter.get(), 0);
    }

    async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }
}
