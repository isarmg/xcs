#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::{Read, ReadBuf};
    use std::pin::Pin;
    use std::time::Duration;

    impl MemRead for &[u8] {
        fn read_mem(&mut self, _: &mut Context<'_>, len: usize) -> Poll<io::Result<Bytes>> {
            let n = std::cmp::min(len, self.len());
            if n > 0 {
                let (a, b) = self.split_at(n);
                let buf = Bytes::copy_from_slice(a);
                *self = b;
                Poll::Ready(Ok(buf))
            } else {
                Poll::Ready(Ok(Bytes::new()))
            }
        }
    }

    impl MemRead for &mut (dyn Read + Unpin) {
        fn read_mem(&mut self, cx: &mut Context<'_>, len: usize) -> Poll<io::Result<Bytes>> {
            let mut v = vec![0; len];
            let mut buf = ReadBuf::new(&mut v);
            ready!(Pin::new(self).poll_read(cx, buf.unfilled())?);
            Poll::Ready(Ok(Bytes::copy_from_slice(buf.filled())))
        }
    }

    impl MemRead for Bytes {
        fn read_mem(&mut self, _: &mut Context<'_>, len: usize) -> Poll<io::Result<Bytes>> {
            let n = std::cmp::min(len, self.len());
            let ret = self.split_to(n);
            Poll::Ready(Ok(ret))
        }
    }

    /*
    use std::io;
    use std::io::Write;
    use super::Decoder;
    use super::ChunkedState;
    use futures::{Async, Poll};
    use bytes::{BytesMut, Bytes};
    use crate::mock::AsyncIo;
    */

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_chunk_size() {
        use std::io::ErrorKind::{InvalidData, InvalidInput, UnexpectedEof};

        async fn read(s: &str) -> u64 {
            let mut state = ChunkedState::new();
            let rdr = &mut s.as_bytes();
            let mut size = 0;
            let mut ext_cnt = 0;
            let mut trailers_cnt = 0;
            loop {
                let result = futures_util::future::poll_fn(|cx| {
                    state.step(
                        cx,
                        rdr,
                        StepArgs {
                            chunk_size: &mut size,
                            extensions_cnt: &mut ext_cnt,
                            chunk_buf: &mut None,
                            trailers_buf: &mut None,
                            trailers_cnt: &mut trailers_cnt,
                            max_headers_cnt: DEFAULT_MAX_HEADERS,
                            max_headers_bytes: TRAILER_LIMIT,
                        },
                    )
                })
                .await;
                let desc = format!("read_size failed for {:?}", s);
                state = result.expect(&desc);
                if state == ChunkedState::Body || state == ChunkedState::EndCr {
                    break;
                }
            }
            size
        }

        async fn read_err(s: &str, expected_err: io::ErrorKind) {
            let mut state = ChunkedState::new();
            let rdr = &mut s.as_bytes();
            let mut size = 0;
            let mut ext_cnt = 0;
            let mut trailers_cnt = 0;
            loop {
                let result = futures_util::future::poll_fn(|cx| {
                    state.step(
                        cx,
                        rdr,
                        StepArgs {
                            chunk_size: &mut size,
                            extensions_cnt: &mut ext_cnt,
                            chunk_buf: &mut None,
                            trailers_buf: &mut None,
                            trailers_cnt: &mut trailers_cnt,
                            max_headers_cnt: DEFAULT_MAX_HEADERS,
                            max_headers_bytes: TRAILER_LIMIT,
                        },
                    )
                })
                .await;
                state = match result {
                    Ok(s) => s,
                    Err(e) => {
                        assert!(
                            expected_err == e.kind(),
                            "Reading {:?}, expected {:?}, but got {:?}",
                            s,
                            expected_err,
                            e.kind()
                        );
                        return;
                    }
                };
                if state == ChunkedState::Body || state == ChunkedState::End {
                    panic!("Was Ok. Expected Err for {:?}", s);
                }
            }
        }

        assert_eq!(1, read("1\r\n").await);
        assert_eq!(1, read("01\r\n").await);
        assert_eq!(0, read("0\r\n").await);
        assert_eq!(0, read("00\r\n").await);
        assert_eq!(10, read("A\r\n").await);
        assert_eq!(10, read("a\r\n").await);
        assert_eq!(255, read("Ff\r\n").await);
        assert_eq!(255, read("Ff   \r\n").await);
        // Missing LF or CRLF
        read_err("F\rF", InvalidInput).await;
        read_err("F", UnexpectedEof).await;
        // Missing digit
        read_err("\r\n\r\n", InvalidInput).await;
        read_err("\r\n", InvalidInput).await;
        // Invalid hex digit
        read_err("X\r\n", InvalidInput).await;
        read_err("1X\r\n", InvalidInput).await;
        read_err("-\r\n", InvalidInput).await;
        read_err("-1\r\n", InvalidInput).await;
        // Acceptable (if not fully valid) extensions do not influence the size
        assert_eq!(1, read("1;extension\r\n").await);
        assert_eq!(10, read("a;ext name=value\r\n").await);
        assert_eq!(1, read("1;extension;extension2\r\n").await);
        assert_eq!(1, read("1;;;  ;\r\n").await);
        assert_eq!(2, read("2; extension...\r\n").await);
        assert_eq!(3, read("3   ; extension=123\r\n").await);
        assert_eq!(3, read("3   ;\r\n").await);
        assert_eq!(3, read("3   ;   \r\n").await);
        // Invalid extensions cause an error
        read_err("1 invalid extension\r\n", InvalidInput).await;
        read_err("1 A\r\n", InvalidInput).await;
        read_err("1;no CRLF", UnexpectedEof).await;
        read_err("1;reject\nnewlines\r\n", InvalidData).await;
        // Overflow
        read_err("f0000000000000003\r\n", InvalidData).await;
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_sized_early_eof() {
        let mut bytes = &b"foo bar"[..];
        let mut decoder = Decoder::length(10);
        assert_eq!(
            decoder
                .decode_fut(&mut bytes)
                .await
                .unwrap()
                .data_ref()
                .unwrap()
                .len(),
            7
        );
        let e = decoder.decode_fut(&mut bytes).await.unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_chunked_early_eof() {
        let mut bytes = &b"\
            9\r\n\
            foo bar\
        "[..];
        let mut decoder = Decoder::chunked(None, None);
        assert_eq!(
            decoder
                .decode_fut(&mut bytes)
                .await
                .unwrap()
                .data_ref()
                .unwrap()
                .len(),
            7
        );
        let e = decoder.decode_fut(&mut bytes).await.unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_chunked_single_read() {
        let mut mock_buf = &b"10\r\n1234567890abcdef\r\n0\r\n"[..];
        let buf = Decoder::chunked(None, None)
            .decode_fut(&mut mock_buf)
            .await
            .expect("decode")
            .into_data()
            .expect("unknown frame type");
        assert_eq!(16, buf.len());
        let result = String::from_utf8(buf.as_ref().to_vec()).expect("decode String");
        assert_eq!("1234567890abcdef", &result);
    }

    #[tokio::test]
    async fn test_read_chunked_with_missing_zero_digit() {
        // After reading a valid chunk, the ending is missing a zero.
        let mut mock_buf = &b"1\r\nZ\r\n\r\n\r\n"[..];
        let mut decoder = Decoder::chunked(None, None);
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect("decode")
            .into_data()
            .expect("unknown frame type");
        assert_eq!("Z", buf);

        let err = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect_err("decode 2");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn test_read_chunked_extensions_over_limit() {
        // construct a chunked body where each individual chunked extension
        // is totally fine, but combined is over the limit.
        let per_chunk = super::CHUNKED_EXTENSIONS_LIMIT * 2 / 3;
        let mut scratch = vec![];
        for _ in 0..2 {
            scratch.extend(b"1;");
            scratch.extend(b"x".repeat(per_chunk as usize));
            scratch.extend(b"\r\nA\r\n");
        }
        scratch.extend(b"0\r\n\r\n");
        let mut mock_buf = Bytes::from(scratch);

        let mut decoder = Decoder::chunked(None, None);
        let buf1 = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect("decode1")
            .into_data()
            .expect("unknown frame type");
        assert_eq!(&buf1[..], b"A");

        let err = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect_err("decode2");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(err.to_string(), "chunk extensions over limit");
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_chunked_trailer_with_missing_lf() {
        let mut mock_buf = &b"10\r\n1234567890abcdef\r\n0\r\nbad\r\r\n"[..];
        let mut decoder = Decoder::chunked(None, None);
        decoder.decode_fut(&mut mock_buf).await.expect("decode");
        let e = decoder.decode_fut(&mut mock_buf).await.unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_chunked_after_eof() {
        let mut mock_buf = &b"10\r\n1234567890abcdef\r\n0\r\n\r\n"[..];
        let mut decoder = Decoder::chunked(None, None);

        // normal read
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .unwrap()
            .into_data()
            .expect("unknown frame type");
        assert_eq!(16, buf.len());
        let result = String::from_utf8(buf.as_ref().to_vec()).expect("decode String");
        assert_eq!("1234567890abcdef", &result);

        // eof read
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect("decode")
            .into_data()
            .expect("unknown frame type");
        assert_eq!(0, buf.len());

        // ensure read after eof also returns eof
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect("decode")
            .into_data()
            .expect("unknown frame type");
        assert_eq!(0, buf.len());
    }

    // perform an async read using a custom buffer size and causing a blocking
    // read at the specified byte
    async fn read_async(mut decoder: Decoder, content: &[u8], block_at: usize) -> String {
        let mut outs = Vec::new();

        let mut ins = crate::common::io::Compat::new(if block_at == 0 {
            tokio_test::io::Builder::new()
                .wait(Duration::from_millis(10))
                .read(content)
                .build()
        } else {
            tokio_test::io::Builder::new()
                .read(&content[..block_at])
                .wait(Duration::from_millis(10))
                .read(&content[block_at..])
                .build()
        });

        let mut ins = &mut ins as &mut (dyn Read + Unpin);

        loop {
            let buf = decoder
                .decode_fut(&mut ins)
                .await
                .expect("unexpected decode error")
                .into_data()
                .expect("unexpected frame type");
            if buf.is_empty() {
                break; // eof
            }
            outs.extend(buf.as_ref());
        }

        String::from_utf8(outs).expect("decode String")
    }

    // iterate over the different ways that this async read could go.
    // tests blocking a read at each byte along the content - The shotgun approach
    async fn all_async_cases(content: &str, expected: &str, decoder: Decoder) {
        let content_len = content.len();
        for block_at in 0..content_len {
            let actual = read_async(decoder.clone(), content.as_bytes(), block_at).await;
            assert_eq!(expected, &actual) //, "Failed async. Blocking at {}", block_at);
        }
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_length_async() {
        let content = "foobar";
        all_async_cases(content, content, Decoder::length(content.len() as u64)).await;
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_chunked_async() {
        let content = "3\r\nfoo\r\n3\r\nbar\r\n0\r\n\r\n";
        let expected = "foobar";
        all_async_cases(content, expected, Decoder::chunked(None, None)).await;
    }

    #[cfg(not(miri))]
    #[tokio::test]
    async fn test_read_eof_async() {
        let content = "foobar";
        all_async_cases(content, content, Decoder::eof()).await;
    }

    #[cfg(all(feature = "nightly", not(miri)))]
    #[bench]
    fn bench_decode_chunked_1kb(b: &mut test::Bencher) {
        let rt = new_runtime();

        const LEN: usize = 1024;
        let mut vec = Vec::new();
        vec.extend(format!("{:x}\r\n", LEN).as_bytes());
        vec.extend(&[0; LEN][..]);
        vec.extend(b"\r\n");
        let content = Bytes::from(vec);

        b.bytes = LEN as u64;

        b.iter(|| {
            let mut decoder = Decoder::chunked(None, None);
            rt.block_on(async {
                let mut raw = content.clone();
                let chunk = decoder
                    .decode_fut(&mut raw)
                    .await
                    .unwrap()
                    .into_data()
                    .unwrap();
                assert_eq!(chunk.len(), LEN);
            });
        });
    }

    #[cfg(all(feature = "nightly", not(miri)))]
    #[bench]
    fn bench_decode_length_1kb(b: &mut test::Bencher) {
        let rt = new_runtime();

        const LEN: usize = 1024;
        let content = Bytes::from(&[0; LEN][..]);
        b.bytes = LEN as u64;

        b.iter(|| {
            let mut decoder = Decoder::length(LEN as u64);
            rt.block_on(async {
                let mut raw = content.clone();
                let chunk = decoder
                    .decode_fut(&mut raw)
                    .await
                    .unwrap()
                    .into_data()
                    .unwrap();
                assert_eq!(chunk.len(), LEN);
            });
        });
    }

    #[cfg(feature = "nightly")]
    fn new_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt build")
    }

    #[test]
    fn test_decode_trailers() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(
            b"Expires: Wed, 21 Oct 2015 07:28:00 GMT\r\nX-Stream-Error: failed to decode\r\n\r\n",
        );
        let headers = decode_trailers(&mut buf, 2).expect("decode_trailers");
        assert_eq!(headers.len(), 2);
        assert_eq!(
            headers.get("Expires").unwrap(),
            "Wed, 21 Oct 2015 07:28:00 GMT"
        );
        assert_eq!(headers.get("X-Stream-Error").unwrap(), "failed to decode");
    }

    #[test]
    fn test_decode_trailers_preserves_duplicate_values() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"X-Trace: first\r\nX-Trace: second\r\n\r\n");

        let headers = decode_trailers(&mut buf, 2).expect("decode_trailers");
        let values = headers
            .get_all("X-Trace")
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(values, ["first", "second"]);
    }

    #[tokio::test]
    async fn test_trailer_max_headers_enforced() {
        let h1_max_headers = 10;
        let mut scratch = vec![];
        scratch.extend(b"10\r\n1234567890abcdef\r\n0\r\n");
        for i in 0..=h1_max_headers {
            scratch.extend(format!("trailer{}: {}\r\n", i, i).as_bytes());
        }
        scratch.extend(b"\r\n");
        let mut mock_buf = Bytes::from(scratch);

        let mut decoder = Decoder::chunked(Some(h1_max_headers), None);

        // ready chunked body
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .unwrap()
            .into_data()
            .expect("unknown frame type");
        assert_eq!(16, buf.len());

        // eof read
        let err = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect_err("trailer fields over limit");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_trailer_max_headers_allows_exact_limit() {
        let h1_max_headers = 10;
        let mut scratch = vec![];
        scratch.extend(b"10\r\n1234567890abcdef\r\n0\r\n");
        for i in 0..h1_max_headers {
            scratch.extend(format!("trailer{}: {}\r\n", i, i).as_bytes());
        }
        scratch.extend(b"\r\n");
        let mut mock_buf = Bytes::from(scratch);

        let mut decoder = Decoder::chunked(Some(h1_max_headers), None);

        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .unwrap()
            .into_data()
            .expect("unknown frame type");
        assert_eq!(16, buf.len());

        let trailers = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect("trailers at limit")
            .into_trailers()
            .expect("unknown frame type");
        assert_eq!(trailers.len(), h1_max_headers);
    }

    #[tokio::test]
    async fn test_trailer_max_header_size_huge_trailer() {
        let max_header_size = 1024;
        let mut scratch = vec![];
        scratch.extend(b"10\r\n1234567890abcdef\r\n0\r\n");
        scratch.extend(format!("huge_trailer: {}\r\n", "x".repeat(max_header_size)).as_bytes());
        scratch.extend(b"\r\n");
        let mut mock_buf = Bytes::from(scratch);

        let mut decoder = Decoder::chunked(None, Some(max_header_size));

        // ready chunked body
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .unwrap()
            .into_data()
            .expect("unknown frame type");
        assert_eq!(16, buf.len());

        // eof read
        let err = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect_err("trailers over limit");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_trailer_max_header_size_many_small_trailers() {
        let max_headers = 10;
        let header_size = 64;
        let mut scratch = vec![];
        scratch.extend(b"10\r\n1234567890abcdef\r\n0\r\n");

        for i in 0..max_headers {
            scratch.extend(format!("trailer{}: {}\r\n", i, "x".repeat(header_size)).as_bytes());
        }

        scratch.extend(b"\r\n");
        let mut mock_buf = Bytes::from(scratch);

        let mut decoder = Decoder::chunked(None, Some(max_headers * header_size));

        // ready chunked body
        let buf = decoder
            .decode_fut(&mut mock_buf)
            .await
            .unwrap()
            .into_data()
            .expect("unknown frame type");
        assert_eq!(16, buf.len());

        // eof read
        let err = decoder
            .decode_fut(&mut mock_buf)
            .await
            .expect_err("trailers over limit");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
