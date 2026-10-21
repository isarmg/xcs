#[cfg(test)]
mod tests {
    use super::*;
    use crate::on_early_drop::OnEarlyDropLayer;
    use bytes::Bytes;
    use http::{Request, Response, StatusCode};
    use http_body_util::{BodyExt, Full};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::time::{sleep, timeout};
    use tower::{service_fn, Layer, ServiceExt};
    use tracing::Span;

    #[derive(Clone, Default)]
    struct RecordingOnFailure {
        events: Arc<Mutex<Vec<DroppedFailure>>>,
    }

    impl OnFailure<DroppedFailure> for RecordingOnFailure {
        fn on_failure(&mut self, class: DroppedFailure, _latency: Duration, _span: &Span) {
            self.events.lock().unwrap().push(class);
        }
    }

    #[tokio::test]
    async fn future_drop_reports_future_failure() {
        let recorder = RecordingOnFailure::default();
        let events = recorder.events.clone();

        let slow_service = service_fn(|_req: Request<()>| async move {
            sleep(Duration::from_secs(60)).await;
            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::new()))
                    .unwrap(),
            )
        });

        let layer = OnEarlyDropLayer::new(EarlyDropsAsFailures::new(recorder));
        let service = layer.layer(slow_service);
        let _ = timeout(
            Duration::from_millis(50),
            service.oneshot(Request::builder().uri("/").body(()).unwrap()),
        )
        .await;

        sleep(Duration::from_millis(10)).await;
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert!(matches!(captured[0], DroppedFailure::Future(_)));
    }

    #[tokio::test]
    async fn body_drop_reports_body_failure_with_status() {
        let recorder = RecordingOnFailure::default();
        let events = recorder.events.clone();

        struct PendingBody;
        impl http_body::Body for PendingBody {
            type Data = Bytes;
            type Error = std::convert::Infallible;
            fn poll_frame(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>>
            {
                std::task::Poll::Pending
            }
            fn is_end_stream(&self) -> bool {
                false
            }
        }

        let service = service_fn(|_req: Request<()>| async move {
            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::CREATED)
                    .body(PendingBody)
                    .unwrap(),
            )
        });

        let layer = OnEarlyDropLayer::new(EarlyDropsAsFailures::new(recorder));
        let service = layer.layer(service);
        let response = service
            .oneshot(Request::builder().uri("/").body(()).unwrap())
            .await
            .unwrap();
        drop(response);

        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1);
        match &captured[0] {
            DroppedFailure::Body(body) => assert_eq!(body.status, StatusCode::CREATED),
            other => panic!("expected Body failure, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn completion_suppresses_both() {
        let recorder = RecordingOnFailure::default();
        let events = recorder.events.clone();

        let ok_service = service_fn(|_req: Request<()>| async move {
            Ok::<_, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from_static(b"hi")))
                    .unwrap(),
            )
        });

        let layer = OnEarlyDropLayer::new(EarlyDropsAsFailures::new(recorder));
        let service = layer.layer(ok_service);
        let response = service
            .oneshot(Request::builder().uri("/").body(()).unwrap())
            .await
            .unwrap();
        let _body = response.into_body().collect().await.unwrap();

        assert!(events.lock().unwrap().is_empty());
    }
}
