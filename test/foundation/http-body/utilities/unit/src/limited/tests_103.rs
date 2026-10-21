#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyExt, Full, StreamBody};
    use bytes::Bytes;
    use std::convert::Infallible;

    #[tokio::test]
    async fn read_for_body_under_limit_returns_data() {
        const DATA: &[u8] = b"testing";
        let inner = Full::new(Bytes::from(DATA));
        let body = &mut Limited::new(inner, 8);

        let mut hint = SizeHint::new();
        hint.set_upper(7);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let data = body.frame().await.unwrap().unwrap().into_data().unwrap();
        assert_eq!(data, DATA);
        hint.set_upper(0);
        assert_eq!(body.size_hint().upper(), hint.upper());

        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn read_for_body_over_limit_returns_error() {
        const DATA: &[u8] = b"testing a string that is too long";
        let inner = Full::new(Bytes::from(DATA));
        let body = &mut Limited::new(inner, 8);

        let mut hint = SizeHint::new();
        hint.set_upper(8);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let error = body.frame().await.unwrap().unwrap_err();
        assert!(matches!(error.downcast_ref(), Some(LengthLimitError)));
    }

    fn body_from_iter<I>(into_iter: I) -> impl Body<Data = Bytes, Error = Infallible>
    where
        I: IntoIterator,
        I::Item: Into<Bytes> + 'static,
        I::IntoIter: Send + 'static,
    {
        let iter = into_iter
            .into_iter()
            .map(|it| Frame::data(it.into()))
            .map(Ok::<_, Infallible>);

        StreamBody::new(futures::stream::iter(iter))
    }

    #[tokio::test]
    async fn read_for_chunked_body_around_limit_returns_first_chunk_but_returns_error_on_over_limit_chunk(
    ) {
        const DATA: [&[u8]; 2] = [b"testing ", b"a string that is too long"];
        let inner = body_from_iter(DATA);
        let body = &mut Limited::new(inner, 8);

        let mut hint = SizeHint::new();
        hint.set_upper(8);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let data = body.frame().await.unwrap().unwrap().into_data().unwrap();
        assert_eq!(data, DATA[0]);
        hint.set_upper(0);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let error = body.frame().await.unwrap().unwrap_err();
        assert!(matches!(error.downcast_ref(), Some(LengthLimitError)));
    }

    #[tokio::test]
    async fn read_for_chunked_body_over_limit_on_first_chunk_returns_error() {
        const DATA: [&[u8]; 2] = [b"testing a string", b" that is too long"];
        let inner = body_from_iter(DATA);
        let body = &mut Limited::new(inner, 8);

        let mut hint = SizeHint::new();
        hint.set_upper(8);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let error = body.frame().await.unwrap().unwrap_err();
        assert!(matches!(error.downcast_ref(), Some(LengthLimitError)));
    }

    #[tokio::test]
    async fn read_for_chunked_body_under_limit_is_okay() {
        const DATA: [&[u8]; 2] = [b"test", b"ing!"];
        let inner = body_from_iter(DATA);
        let body = &mut Limited::new(inner, 8);

        let mut hint = SizeHint::new();
        hint.set_upper(8);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let data = body.frame().await.unwrap().unwrap().into_data().unwrap();
        assert_eq!(data, DATA[0]);
        hint.set_upper(4);
        assert_eq!(body.size_hint().upper(), hint.upper());

        let data = body.frame().await.unwrap().unwrap().into_data().unwrap();
        assert_eq!(data, DATA[1]);
        hint.set_upper(0);
        assert_eq!(body.size_hint().upper(), hint.upper());

        assert!(body.frame().await.is_none());
    }

    struct SomeTrailers;

    impl Body for SomeTrailers {
        type Data = Bytes;
        type Error = Infallible;

        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            Poll::Ready(Some(Ok(Frame::trailers(http::HeaderMap::new()))))
        }
    }

    #[tokio::test]
    async fn read_for_trailers_propagates_inner_trailers() {
        let body = &mut Limited::new(SomeTrailers, 8);
        let frame = body.frame().await.unwrap().unwrap();
        assert!(frame.is_trailers());
    }

    #[derive(Debug)]
    struct ErrorBodyError;

    impl fmt::Display for ErrorBodyError {
        fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
            Ok(())
        }
    }

    impl Error for ErrorBodyError {}

    struct ErrorBody;

    impl Body for ErrorBody {
        type Data = &'static [u8];
        type Error = ErrorBodyError;

        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            Poll::Ready(Some(Err(ErrorBodyError)))
        }
    }

    #[tokio::test]
    async fn read_for_body_returning_error_propagates_error() {
        let body = &mut Limited::new(ErrorBody, 8);
        let error = body.frame().await.unwrap().unwrap_err();
        assert!(matches!(error.downcast_ref(), Some(ErrorBodyError)));
    }
}
