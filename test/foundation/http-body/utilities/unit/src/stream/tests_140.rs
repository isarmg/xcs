#[cfg(test)]
mod tests {
    use crate::{BodyExt, BodyStream, StreamBody};
    use bytes::Bytes;
    use futures_util::StreamExt;
    use crate::Frame;
    use std::convert::Infallible;

    #[tokio::test]
    async fn body_from_stream() {
        let chunks: Vec<Result<_, Infallible>> = vec![
            Ok(Frame::data(Bytes::from(vec![1]))),
            Ok(Frame::data(Bytes::from(vec![2]))),
            Ok(Frame::data(Bytes::from(vec![3]))),
        ];
        let stream = futures_util::stream::iter(chunks);
        let mut body = StreamBody::new(stream);

        assert_eq!(
            body.frame()
                .await
                .unwrap()
                .unwrap()
                .into_data()
                .unwrap()
                .as_ref(),
            [1]
        );
        assert_eq!(
            body.frame()
                .await
                .unwrap()
                .unwrap()
                .into_data()
                .unwrap()
                .as_ref(),
            [2]
        );
        assert_eq!(
            body.frame()
                .await
                .unwrap()
                .unwrap()
                .into_data()
                .unwrap()
                .as_ref(),
            [3]
        );

        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn stream_from_body() {
        let chunks: Vec<Result<_, Infallible>> = vec![
            Ok(Frame::data(Bytes::from(vec![1]))),
            Ok(Frame::data(Bytes::from(vec![2]))),
            Ok(Frame::data(Bytes::from(vec![3]))),
        ];
        let stream = futures_util::stream::iter(chunks);
        let body = StreamBody::new(stream);

        let mut stream = BodyStream::new(body);

        assert_eq!(
            stream
                .next()
                .await
                .unwrap()
                .unwrap()
                .into_data()
                .unwrap()
                .as_ref(),
            [1]
        );
        assert_eq!(
            stream
                .next()
                .await
                .unwrap()
                .unwrap()
                .into_data()
                .unwrap()
                .as_ref(),
            [2]
        );
        assert_eq!(
            stream
                .next()
                .await
                .unwrap()
                .unwrap()
                .into_data()
                .unwrap()
                .as_ref(),
            [3]
        );

        assert!(stream.next().await.is_none());
    }
}
