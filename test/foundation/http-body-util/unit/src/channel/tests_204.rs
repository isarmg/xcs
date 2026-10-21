#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http::{HeaderName, HeaderValue};

    use crate::BodyExt;

    use super::*;

    #[tokio::test]
    async fn empty() {
        let (tx, body) = Channel::<Bytes>::new(1024);
        drop(tx);

        let collected = body.collect().await.unwrap();
        assert!(collected.trailers().is_none());
        assert!(collected.to_bytes().is_empty());
    }

    #[tokio::test]
    async fn can_send_data() {
        let (mut tx, body) = Channel::<Bytes>::new(1024);

        tokio::spawn(async move {
            tx.send_data(Bytes::from("Hel")).await.unwrap();
            tx.send_data(Bytes::from("lo!")).await.unwrap();
        });

        let collected = body.collect().await.unwrap();
        assert!(collected.trailers().is_none());
        assert_eq!(collected.to_bytes(), "Hello!");
    }

    #[tokio::test]
    async fn can_send_trailers() {
        let (mut tx, body) = Channel::<Bytes>::new(1024);

        tokio::spawn(async move {
            let mut trailers = HeaderMap::new();
            trailers.insert(
                HeaderName::from_static("foo"),
                HeaderValue::from_static("bar"),
            );
            tx.send_trailers(trailers).await.unwrap();
        });

        let collected = body.collect().await.unwrap();
        assert_eq!(collected.trailers().unwrap()["foo"], "bar");
        assert!(collected.to_bytes().is_empty());
    }

    #[tokio::test]
    async fn can_send_both_data_and_trailers() {
        let (mut tx, body) = Channel::<Bytes>::new(1024);

        tokio::spawn(async move {
            tx.send_data(Bytes::from("Hel")).await.unwrap();
            tx.send_data(Bytes::from("lo!")).await.unwrap();
            let mut trailers = HeaderMap::new();
            trailers.insert(
                HeaderName::from_static("foo"),
                HeaderValue::from_static("bar"),
            );
            tx.send_trailers(trailers).await.unwrap();
        });

        let collected = body.collect().await.unwrap();
        assert_eq!(collected.trailers().unwrap()["foo"], "bar");
        assert_eq!(collected.to_bytes(), "Hello!");
    }

    #[tokio::test]
    async fn try_send_works() {
        let (mut tx, mut body) = Channel::<Bytes>::new(2);

        // Send two messages, filling the channel's buffer.
        tx.try_send(Frame::data(Bytes::from("one")))
            .expect("can send one message");
        tx.try_send(Frame::data(Bytes::from("two")))
            .expect("can send two messages");

        // Sending a value to a full channel should return it back to us.
        match tx.try_send(Frame::data(Bytes::from("three"))) {
            Err(frame) => assert_eq!(frame.into_data().unwrap(), "three"),
            Ok(()) => panic!("synchronously sending a value to a full channel should fail"),
        };

        // Read the messages out of the body.
        assert_eq!(
            body.frame()
                .await
                .expect("yields result")
                .expect("yields frame")
                .into_data()
                .expect("yields data"),
            "one"
        );
        assert_eq!(
            body.frame()
                .await
                .expect("yields result")
                .expect("yields frame")
                .into_data()
                .expect("yields data"),
            "two"
        );

        // Drop the body.
        drop(body);

        // Sending a value to a closed channel should return it back to us.
        match tx.try_send(Frame::data(Bytes::from("closed"))) {
            Err(frame) => assert_eq!(frame.into_data().unwrap(), "closed"),
            Ok(()) => panic!("synchronously sending a value to a closed channel should fail"),
        };
    }

    /// A stand-in for an error type, for unit tests.
    type Error = &'static str;
    /// An example error message.
    const MSG: Error = "oh no";

    #[tokio::test]
    async fn aborts_before_trailers() {
        let (mut tx, body) = Channel::<Bytes, Error>::new(1024);

        tokio::spawn(async move {
            tx.send_data(Bytes::from("Hel")).await.unwrap();
            tx.send_data(Bytes::from("lo!")).await.unwrap();
            tx.abort(MSG);
        });

        let err = body.collect().await.unwrap_err();
        assert_eq!(err, MSG);
    }

    #[tokio::test]
    async fn aborts_after_trailers() {
        let (mut tx, body) = Channel::<Bytes, Error>::new(1024);

        tokio::spawn(async move {
            tx.send_data(Bytes::from("Hel")).await.unwrap();
            tx.send_data(Bytes::from("lo!")).await.unwrap();
            let mut trailers = HeaderMap::new();
            trailers.insert(
                HeaderName::from_static("foo"),
                HeaderValue::from_static("bar"),
            );
            tx.send_trailers(trailers).await.unwrap();
            tx.abort(MSG);
        });

        let err = body.collect().await.unwrap_err();
        assert_eq!(err, MSG);
    }
}
