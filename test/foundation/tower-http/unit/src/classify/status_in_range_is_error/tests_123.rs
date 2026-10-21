#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use http::Response;

    #[test]
    fn basic() {
        let classifier = StatusInRangeAsFailures::new(400..=599);

        assert!(matches!(
            classifier
                .clone()
                .classify_response(&response_with_status(200)),
            ClassifiedResponse::Ready(Ok(())),
        ));

        assert!(matches!(
            classifier
                .clone()
                .classify_response(&response_with_status(400)),
            ClassifiedResponse::Ready(Err(StatusInRangeFailureClass::StatusCode(
                StatusCode::BAD_REQUEST
            ))),
        ));

        assert!(matches!(
            classifier.classify_response(&response_with_status(500)),
            ClassifiedResponse::Ready(Err(StatusInRangeFailureClass::StatusCode(
                StatusCode::INTERNAL_SERVER_ERROR
            ))),
        ));
    }

    fn response_with_status(status: u16) -> Response<()> {
        Response::builder().status(status).body(()).unwrap()
    }
}
