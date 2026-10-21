#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use bytes::Bytes;
    use http::{HeaderName, HeaderValue};

    use crate::{BodyExt, Empty, Full};

    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn works() {
        let mut trailers = HeaderMap::new();
        trailers.insert(
            HeaderName::from_static("foo"),
            HeaderValue::from_static("bar"),
        );

        let body =
            Full::<Bytes>::from("hello").with_trailers(std::future::ready(Some(
                Ok::<_, Infallible>(trailers.clone()),
            )));

        futures_util::pin_mut!(body);
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let data = unwrap_ready(body.as_mut().poll_frame(&mut cx))
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        assert_eq!(data, "hello");

        let body_trailers = unwrap_ready(body.as_mut().poll_frame(&mut cx))
            .unwrap()
            .unwrap()
            .into_trailers()
            .unwrap();
        assert_eq!(body_trailers, trailers);

        assert!(unwrap_ready(body.as_mut().poll_frame(&mut cx)).is_none());
    }

    #[tokio::test]
    async fn merges_trailers() {
        let mut trailers_1 = HeaderMap::new();
        trailers_1.insert(
            HeaderName::from_static("foo"),
            HeaderValue::from_static("bar"),
        );

        let mut trailers_2 = HeaderMap::new();
        trailers_2.insert(
            HeaderName::from_static("baz"),
            HeaderValue::from_static("qux"),
        );

        let body = Empty::<Bytes>::new()
            .with_trailers(std::future::ready(Some(Ok::<_, Infallible>(
                trailers_1.clone(),
            ))))
            .with_trailers(std::future::ready(Some(Ok::<_, Infallible>(
                trailers_2.clone(),
            ))));

        futures_util::pin_mut!(body);
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let body_trailers = unwrap_ready(body.as_mut().poll_frame(&mut cx))
            .unwrap()
            .unwrap()
            .into_trailers()
            .unwrap();

        let mut all_trailers = HeaderMap::new();
        all_trailers.extend(trailers_1);
        all_trailers.extend(trailers_2);
        assert_eq!(body_trailers, all_trailers);

        assert!(unwrap_ready(body.as_mut().poll_frame(&mut cx)).is_none());
    }

    fn unwrap_ready<T>(poll: Poll<T>) -> T {
        match poll {
            Poll::Ready(t) => t,
            Poll::Pending => panic!("pending"),
        }
    }
}
