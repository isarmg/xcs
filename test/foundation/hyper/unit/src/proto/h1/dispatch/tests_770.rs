#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::io::Compat;
    use crate::proto::h1::ClientTransaction;
    use std::time::Duration;

    #[test]
    fn client_read_bytes_before_writing_request() {
        let _ = pretty_env_logger::try_init();

        tokio_test::task::spawn(()).enter(|cx, _| {
            let (io, mut handle) = tokio_test::io::Builder::new().build_with_handle();

            // Block at 0 for now, but we will release this response before
            // the request is ready to write later...
            let (mut tx, rx) = crate::client::dispatch::channel();
            let conn = Conn::<_, bytes::Bytes, ClientTransaction>::new(Compat::new(io));
            let mut dispatcher = Dispatcher::new(Client::new(rx), conn);

            // First poll is needed to allow tx to send...
            assert!(Pin::new(&mut dispatcher).poll(cx).is_pending());

            // Unblock our IO, which has a response before we've sent request!
            //
            handle.read(b"HTTP/1.1 200 OK\r\n\r\n");

            let mut res_rx = tx
                .try_send(crate::Request::new(IncomingBody::empty()))
                .unwrap();

            tokio_test::assert_ready_ok!(Pin::new(&mut dispatcher).poll(cx));
            let err = tokio_test::assert_ready_ok!(Pin::new(&mut res_rx).poll(cx))
                .expect_err("callback should send error");

            match (err.error.is_canceled(), err.message.as_ref()) {
                (true, Some(_)) => (),
                _ => panic!("expected Canceled, got {:?}", err),
            }
        });
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn client_flushing_is_not_ready_for_next_request() {
        let _ = pretty_env_logger::try_init();

        let (io, _handle) = tokio_test::io::Builder::new()
            .write(b"POST / HTTP/1.1\r\ncontent-length: 4\r\n\r\n")
            .read(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .wait(std::time::Duration::from_secs(2))
            .build_with_handle();

        let (mut tx, rx) = crate::client::dispatch::channel();
        let mut conn = Conn::<_, bytes::Bytes, ClientTransaction>::new(Compat::new(io));
        conn.set_write_strategy_queue();

        let dispatcher = Dispatcher::new(Client::new(rx), conn);
        let _dispatcher = tokio::spawn(async move { dispatcher.await });

        let body = {
            let (mut tx, body) = IncomingBody::new_channel(DecodedLength::new(4), false);
            tx.try_send_data("reee".into()).unwrap();
            body
        };

        let req = crate::Request::builder().method("POST").body(body).unwrap();

        let res = tx.try_send(req).unwrap().await.expect("response");
        drop(res);

        assert!(!tx.is_ready());
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn body_empty_chunks_ignored() {
        let _ = pretty_env_logger::try_init();

        let io = tokio_test::io::Builder::new()
            // no reading or writing, just be blocked for the test...
            .wait(Duration::from_secs(5))
            .build();

        let (mut tx, rx) = crate::client::dispatch::channel();
        let conn = Conn::<_, bytes::Bytes, ClientTransaction>::new(Compat::new(io));
        let mut dispatcher = tokio_test::task::spawn(Dispatcher::new(Client::new(rx), conn));

        // First poll is needed to allow tx to send...
        assert!(dispatcher.poll().is_pending());

        let body = {
            let (mut tx, body) = IncomingBody::channel();
            tx.try_send_data("".into()).unwrap();
            body
        };

        let _res_rx = tx.try_send(crate::Request::new(body)).unwrap();

        // Ensure conn.write_body wasn't called with the empty chunk.
        // If it is, it will trigger an assertion.
        assert!(dispatcher.poll().is_pending());
    }
}
