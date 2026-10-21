#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_display() {
        assert_eq!(
            DroppedFailure::Future(FutureDropped).to_string(),
            "response future dropped before completion",
        );
    }

    #[test]
    fn body_display_carries_status() {
        assert_eq!(
            DroppedFailure::Body(BodyDropped {
                status: StatusCode::INTERNAL_SERVER_ERROR
            })
            .to_string(),
            "response body dropped before end-of-stream (status: 500 Internal Server Error)",
        );
    }
}
