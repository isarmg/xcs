#[cfg(test)]
mod tests {
    #[cfg(all(feature = "nightly", not(miri)))]
    #[bench]
    fn bench_read_head_short(b: &mut ::test::Bencher) {
        use super::*;
        use crate::common::io::Compat;
        let s = b"GET / HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let len = s.len();
        b.bytes = len as u64;

        // an empty IO, we'll be skipping and using the read buffer anyways
        let io = Compat(tokio_test::io::Builder::new().build());
        let mut conn = Conn::<_, bytes::Bytes, crate::proto::h1::ServerTransaction>::new(io);
        *conn.io.read_buf_mut() = ::bytes::BytesMut::from(&s[..]);
        conn.state.cached_headers = Some(HeaderMap::with_capacity(2));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        b.iter(|| {
            rt.block_on(futures::future::poll_fn(|cx| {
                match conn.poll_read_head(cx) {
                    Poll::Ready(Some(Ok(x))) => {
                        ::test::black_box(&x);
                        let mut headers = x.0.headers;
                        headers.clear();
                        conn.state.cached_headers = Some(headers);
                    }
                    f => panic!("expected Ready(Some(Ok(..))): {:?}", f),
                }

                conn.io.read_buf_mut().reserve(1);
                unsafe {
                    conn.io.read_buf_mut().set_len(len);
                }
                conn.state.reading = Reading::Init;
                Poll::Ready(())
            }));
        });
    }

    // A client request carrying `Connection: close` must evict the connection
    // (disable keep-alive) at request-encode time, so it is never returned to the
    // pool for reuse — independent of whether the backend response echoes
    // `Connection: close`. hyper otherwise derives reuse from the response alone.
    #[cfg(feature = "client")]
    #[test]
    fn client_request_connection_close_disables_keep_alive() {
        use super::*;
        use crate::common::io::Compat;
        use crate::proto::RequestLine;

        // Encodes a client GET (with the given Connection header lines, one
        // `append` each) on a fresh client Conn and returns whether the
        // connection remains reusable.
        fn remains_reusable_after_get(connection_values: &[&'static str]) -> bool {
            let io = Compat(tokio_test::io::Builder::new().build());
            let mut conn = Conn::<_, bytes::Bytes, crate::proto::h1::ClientTransaction>::new(io);
            assert!(
                conn.state.wants_keep_alive(),
                "a fresh client connection should want keep-alive"
            );

            let mut headers = HeaderMap::new();
            for value in connection_values {
                headers.append(CONNECTION, HeaderValue::from_static(value));
            }
            let head = MessageHead {
                version: Version::HTTP_11,
                subject: RequestLine(Method::GET, "/".parse().unwrap()),
                headers,
                extensions: http::Extensions::new(),
            };
            conn.write_head(head, None);
            conn.state.wants_keep_alive()
        }

        // Control: a request without `Connection: close` leaves the connection reusable.
        assert!(
            remains_reusable_after_get(&[]),
            "a keep-alive request must leave the connection reusable"
        );
        // Fix: a `Connection: close` request must disable keep-alive so the
        // connection is evicted regardless of the response.
        assert!(
            !remains_reusable_after_get(&["close"]),
            "a `Connection: close` request must disable keep-alive (connection evicted)"
        );
        // A `close` token in a comma-separated value is honored.
        assert!(
            !remains_reusable_after_get(&["keep-alive, close"]),
            "a `close` token in a comma-separated Connection value must disable keep-alive"
        );
        // A `close` in ANY of multiple Connection header lines is honored
        // (get_all, not just the first line).
        assert!(
            !remains_reusable_after_get(&["keep-alive", "close"]),
            "a `close` in any Connection header line must disable keep-alive"
        );
    }

    use super::*;
    use crate::common::io::Compat;
    #[cfg(feature = "client")]
    use crate::proto::h1::ClientTransaction;
    #[cfg(feature = "server")]
    use crate::proto::h1::ServerTransaction;
    #[cfg(feature = "server")]
    use crate::proto::RequestLine;
    use bytes::Bytes;

    fn poll_head<I, T>(
        conn: &mut Conn<Compat<I>, Bytes, T>,
    ) -> Poll<Option<crate::Result<(MessageHead<T::Incoming>, DecodedLength, Wants)>>>
    where
        I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        T: Http1Transaction + Unpin,
    {
        tokio_test::task::spawn(()).enter(|cx, _| conn.poll_read_head(cx))
    }

    fn ready<T>(poll: Poll<T>) -> T {
        match poll {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("expected ready"),
        }
    }

    #[cfg(feature = "server")]
    #[test]
    fn conn_reads_request_head() {
        let io = tokio_test::io::Builder::new()
            .read(b"GET / HTTP/1.1\r\n\r\n")
            .build();
        let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));

        let (head, body, _) = ready(poll_head(&mut conn))
            .expect("message")
            .expect("valid request");
        assert_eq!(head.subject, RequestLine(Method::GET, "/".parse().unwrap()));
        assert_eq!(body, DecodedLength::ZERO);
    }

    #[cfg(feature = "server")]
    #[test]
    fn conn_reads_partial_request_head() {
        tokio_test::task::spawn(()).enter(|cx, _| {
            let (io, mut handle) = tokio_test::io::Builder::new().build_with_handle();
            let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
            handle.read(b"GET / HTTP");
            assert!(conn.poll_read_head(cx).is_pending());
            handle.read(b"/1.1\r\nHost: foo.bar\r\n\r\n");
            assert!(conn.poll_read_head(cx).is_ready());
        });
    }

    #[cfg(feature = "server")]
    #[test]
    fn conn_accepts_eof_when_idle() {
        let io = tokio_test::io::Builder::new().build();
        let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
        conn.state.idle::<ServerTransaction>();
        assert!(matches!(poll_head(&mut conn), Poll::Ready(None)));
    }

    #[cfg(feature = "server")]
    #[test]
    fn conn_rejects_eof_during_partial_head() {
        let io = tokio_test::io::Builder::new()
            .read(b"GET / HTTP/1.1")
            .build();
        let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
        conn.state.idle::<ServerTransaction>();
        let err = ready(poll_head(&mut conn))
            .expect("error result")
            .expect_err("partial head must fail");
        assert!(err.is_incomplete_message(), "unexpected error: {err:?}");
    }

    #[cfg(feature = "client")]
    #[test]
    fn client_rejects_eof_while_busy() {
        let io = tokio_test::io::Builder::new().build();
        let mut client = Conn::<_, Bytes, ClientTransaction>::new(Compat::new(io));
        client.state.busy();
        client.state.writing = Writing::KeepAlive;
        let err = ready(poll_head(&mut client))
            .expect("error result")
            .expect_err("client EOF must fail");
        assert!(err.is_incomplete_message(), "unexpected error: {err:?}");
    }

    #[cfg(feature = "server")]
    #[test]
    fn server_accepts_eof_while_busy() {
        let io = tokio_test::io::Builder::new().build();
        let mut server = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
        server.state.busy();
        assert!(matches!(poll_head(&mut server), Poll::Ready(None)));
    }

    #[cfg(feature = "client")]
    #[test]
    fn conn_reads_empty_response_before_eof() {
        let io = tokio_test::io::Builder::new()
            .read(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .build();
        let mut conn = Conn::<_, Bytes, ClientTransaction>::new(Compat::new(io));
        conn.state.busy();
        conn.state.writing = Writing::KeepAlive;
        let (_, body, _) = ready(poll_head(&mut conn))
            .expect("response")
            .expect("valid response");
        assert_eq!(body, DecodedLength::ZERO);
    }

    #[cfg(feature = "server")]
    #[test]
    fn conn_reads_body_and_reports_end() {
        let io = tokio_test::io::Builder::new()
            .read(b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\n12345")
            .wait(std::time::Duration::from_secs(1))
            .build();
        let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
        let (_, body, _) = ready(poll_head(&mut conn))
            .expect("request")
            .expect("valid request");
        assert_eq!(body, DecodedLength::new(5));

        tokio_test::task::spawn(()).enter(|cx, _| {
            let frame = conn.poll_read_body(cx);
            let data = ready(frame)
                .expect("body frame")
                .expect("valid body")
                .into_data()
                .expect("data frame");
            assert_eq!(data, "12345");
            assert!(
                !conn.can_read_body(),
                "the complete body must return to head-reading state"
            );
        });
    }

    #[cfg(feature = "server")]
    #[test]
    fn closed_conn_cannot_read_or_write() {
        let io = tokio_test::io::Builder::new().build();
        let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
        conn.state.close();
        assert!(conn.is_read_closed());
        assert!(conn.is_write_closed());
        assert!(!conn.can_read_head());
        assert!(!conn.can_write_head());
    }

    #[cfg(feature = "server")]
    #[test]
    fn conn_writes_chunked_body() {
        let io = tokio_test::io::Builder::new()
            .write(b"7\r\nheaders\r\n0\r\n\r\n")
            .build();
        let mut conn = Conn::<_, Bytes, ServerTransaction>::new(Compat::new(io));
        conn.state.writing = Writing::Body(Encoder::chunked());
        conn.write_body(Bytes::from_static(b"headers"));
        conn.end_body().unwrap();
        tokio_test::task::spawn(()).enter(|cx, _| {
            assert!(conn.poll_flush(cx).is_ready());
        });
    }
}
