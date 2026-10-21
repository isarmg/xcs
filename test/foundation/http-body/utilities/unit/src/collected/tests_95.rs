#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use futures::stream;

    use crate::{BodyExt, Full, StreamBody};

    use super::*;

    #[tokio::test]
    async fn full_body() {
        let body = Full::new(&b"hello"[..]);

        let buffered = body.collect().await.unwrap();

        let mut buf = buffered.to_bytes();

        assert_eq!(&buf.copy_to_bytes(buf.remaining())[..], &b"hello"[..]);
    }

    #[tokio::test]
    async fn segmented_body() {
        let bufs = [&b"hello"[..], &b"world"[..], &b"!"[..]];
        let body = StreamBody::new(stream::iter(bufs.map(Frame::data).map(Ok::<_, Infallible>)));

        let buffered = body.collect().await.unwrap();

        let mut buf = buffered.to_bytes();

        assert_eq!(&buf.copy_to_bytes(buf.remaining())[..], b"helloworld!");
    }

    #[tokio::test]
    async fn delayed_segments() {
        let one = stream::once(async { Ok::<_, Infallible>(Frame::data(&b"hello "[..])) });
        let two = stream::once(async {
            // a yield just so its not ready immediately
            tokio::task::yield_now().await;
            Ok::<_, Infallible>(Frame::data(&b"world!"[..]))
        });
        let stream = futures::StreamExt::chain(one, two);

        let body = StreamBody::new(stream);

        let buffered = body.collect().await.unwrap();

        let mut buf = buffered.to_bytes();

        assert_eq!(&buf.copy_to_bytes(buf.remaining())[..], b"hello world!");
    }

    #[tokio::test]
    async fn trailers() {
        let mut trailers = HeaderMap::new();
        trailers.insert("this", "a trailer".try_into().unwrap());
        let bufs = [
            Frame::data(&b"hello"[..]),
            Frame::data(&b"world!"[..]),
            Frame::trailers(trailers.clone()),
        ];

        let body = StreamBody::new(stream::iter(bufs.map(Ok::<_, Infallible>)));

        let buffered = body.collect().await.unwrap();

        assert_eq!(&trailers, buffered.trailers().unwrap());

        let mut buf = buffered.to_bytes();

        assert_eq!(&buf.copy_to_bytes(buf.remaining())[..], b"helloworld!");
    }

    /// Test for issue [#88](https://github.com/hyperium/http-body/issues/88).
    #[tokio::test]
    async fn empty_frame() {
        let bufs: [&[u8]; 1] = [&[]];

        let body = StreamBody::new(stream::iter(bufs.map(Frame::data).map(Ok::<_, Infallible>)));
        let buffered = body.collect().await.unwrap();

        assert_eq!(buffered.to_bytes().len(), 0);
    }
}
