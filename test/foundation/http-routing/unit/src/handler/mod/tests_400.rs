#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extract::State, test_helpers::*};
    use http_routing::kernel::body::Body;
    use http::StatusCode;
    use std::time::Duration;
    use tower_http::{
        limit::RequestBodyLimitLayer, map_request_body::MapRequestBodyLayer,
        map_response_body::MapResponseBodyLayer, timeout::TimeoutLayer,
    };

    #[crate::test]
    async fn handler_into_service() {
        async fn handle(body: String) -> impl IntoResponse {
            format!("you said: {body}")
        }

        let client = TestClient::new(handle.into_service());

        let res = client.post("/").body("hi there!").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "you said: hi there!");
    }

    #[crate::test]
    async fn with_layer_that_changes_request_body_and_state() {
        async fn handle(State(state): State<&'static str>) -> &'static str {
            state
        }

        let svc = handle
            .layer((
                RequestBodyLimitLayer::new(1024),
                TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(10),
                ),
                MapResponseBodyLayer::new(Body::new),
            ))
            .layer(MapRequestBodyLayer::new(Body::new))
            .with_state("foo");

        let client = TestClient::new(svc);
        let res = client.get("/").await;
        assert_eq!(res.text().await, "foo");
    }
}
