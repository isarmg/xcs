#[cfg(test)]
mod tests {
    use super::*;

    use bytes::Bytes;
    use http_body::Frame;
    use http_body_util::BodyExt;
    use pin_project_lite::pin_project;
    use std::{error::Error, fmt::Display};

    #[derive(Debug)]
    struct MockError;

    impl Error for MockError {}

    impl Display for MockError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "mock error")
        }
    }

    pin_project! {
        struct MockBody {
            #[pin]
            sleep: Sleep
        }
    }

    impl Body for MockBody {
        type Data = Bytes;
        type Error = MockError;

        fn poll_frame(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
            let this = self.project();
            this.sleep
                .poll(cx)
                .map(|_| Some(Ok(Frame::data(vec![].into()))))
        }
    }

    #[tokio::test]
    async fn test_body_available_within_timeout() {
        let mock_sleep = Duration::from_secs(1);
        let timeout_sleep = Duration::from_secs(2);

        let mock_body = MockBody {
            sleep: sleep(mock_sleep),
        };
        let timeout_body = TimeoutBody::new(timeout_sleep, mock_body);

        assert!(timeout_body
            .boxed()
            .frame()
            .await
            .expect("no frame")
            .is_ok());
    }

    #[tokio::test]
    async fn test_body_unavailable_within_timeout_error() {
        let mock_sleep = Duration::from_secs(2);
        let timeout_sleep = Duration::from_secs(1);

        let mock_body = MockBody {
            sleep: sleep(mock_sleep),
        };
        let timeout_body = TimeoutBody::new(timeout_sleep, mock_body);

        assert!(timeout_body.boxed().frame().await.unwrap().is_err());
    }
}
