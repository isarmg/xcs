#[cfg(test)]
mod tests {
    use bytes::BufMut;
    use http::{
        header::{
            AUTHORIZATION, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE,
            CONTENT_TYPE, HOST, MAX_FORWARDS, SET_COOKIE, TE, TRAILER, TRANSFER_ENCODING,
        },
        HeaderMap, HeaderName, HeaderValue,
    };

    use super::super::io::Cursor;
    use super::Encoder;

    #[test]
    fn chunked() {
        let mut encoder = Encoder::chunked();
        let mut dst = Vec::new();

        let msg1 = b"foo bar".as_ref();
        let buf1 = encoder.encode(msg1);
        dst.put(buf1);
        assert_eq!(dst, b"7\r\nfoo bar\r\n");

        let msg2 = b"baz quux herp".as_ref();
        let buf2 = encoder.encode(msg2);
        dst.put(buf2);

        assert_eq!(dst, b"7\r\nfoo bar\r\nD\r\nbaz quux herp\r\n");

        let end = encoder.end::<Cursor<Vec<u8>>>().unwrap().unwrap();
        dst.put(end);

        assert_eq!(
            dst,
            b"7\r\nfoo bar\r\nD\r\nbaz quux herp\r\n0\r\n\r\n".as_ref()
        );
    }

    #[test]
    fn length() {
        let max_len = 8;
        let mut encoder = Encoder::length(max_len as u64);
        let mut dst = Vec::new();

        let msg1 = b"foo bar".as_ref();
        let buf1 = encoder.encode(msg1);
        dst.put(buf1);

        assert_eq!(dst, b"foo bar");
        assert!(!encoder.is_eof());
        encoder.end::<()>().unwrap_err();

        let msg2 = b"baz".as_ref();
        let buf2 = encoder.encode(msg2);
        dst.put(buf2);

        assert_eq!(dst.len(), max_len);
        assert_eq!(dst, b"foo barb");
        assert!(encoder.is_eof());
        assert!(encoder.end::<()>().unwrap().is_none());
    }

    #[cfg(feature = "server")]
    #[test]
    fn eof() {
        let mut encoder = Encoder::close_delimited();
        let mut dst = Vec::new();

        let msg1 = b"foo bar".as_ref();
        let buf1 = encoder.encode(msg1);
        dst.put(buf1);

        assert_eq!(dst, b"foo bar");
        assert!(!encoder.is_eof());
        encoder.end::<()>().unwrap();

        let msg2 = b"baz".as_ref();
        let buf2 = encoder.encode(msg2);
        dst.put(buf2);

        assert_eq!(dst, b"foo barbaz");
        assert!(!encoder.is_eof());
        encoder.end::<()>().unwrap();
    }

    #[test]
    fn chunked_with_valid_trailers() {
        let encoder = Encoder::chunked();
        let trailers = vec![HeaderName::from_static("chunky-trailer")];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        let headers = HeaderMap::from_iter(vec![
            (
                HeaderName::from_static("chunky-trailer"),
                HeaderValue::from_static("header data"),
            ),
            (
                HeaderName::from_static("should-not-be-included"),
                HeaderValue::from_static("oops"),
            ),
        ]);

        let buf1 = encoder.encode_trailers::<&[u8]>(headers, false).unwrap();

        let mut dst = Vec::new();
        dst.put(buf1);
        assert_eq!(dst, b"0\r\nchunky-trailer: header data\r\n\r\n");
    }

    #[test]
    fn chunked_with_multiple_trailer_headers() {
        let encoder = Encoder::chunked();
        let trailers = vec![
            HeaderName::from_static("chunky-trailer"),
            HeaderName::from_static("chunky-trailer-2"),
        ];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        let headers = HeaderMap::from_iter(vec![
            (
                HeaderName::from_static("chunky-trailer"),
                HeaderValue::from_static("header data"),
            ),
            (
                HeaderName::from_static("chunky-trailer-2"),
                HeaderValue::from_static("more header data"),
            ),
        ]);

        let buf1 = encoder.encode_trailers::<&[u8]>(headers, false).unwrap();

        let mut dst = Vec::new();
        dst.put(buf1);
        assert_eq!(
            dst,
            b"0\r\nchunky-trailer: header data\r\nchunky-trailer-2: more header data\r\n\r\n"
        );
    }

    #[test]
    fn chunked_with_duplicate_trailer_values() {
        let encoder = Encoder::chunked();
        let trailers = vec![HeaderName::from_static("chunky-trailer")];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_static("chunky-trailer"),
            HeaderValue::from_static("first"),
        );
        headers.append(
            HeaderName::from_static("chunky-trailer"),
            HeaderValue::from_static("second"),
        );

        let buf1 = encoder.encode_trailers::<&[u8]>(headers, false).unwrap();

        let mut dst = Vec::new();
        dst.put(buf1);
        assert_eq!(
            dst,
            b"0\r\nchunky-trailer: first\r\nchunky-trailer: second\r\n\r\n"
        );
    }

    #[test]
    fn chunked_with_no_trailer_header() {
        let encoder = Encoder::chunked();

        let headers = HeaderMap::from_iter(vec![(
            HeaderName::from_static("chunky-trailer"),
            HeaderValue::from_static("header data"),
        )]);

        assert!(encoder
            .encode_trailers::<&[u8]>(headers.clone(), false)
            .is_none());

        let trailers = vec![];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        assert!(encoder.encode_trailers::<&[u8]>(headers, false).is_none());
    }

    #[test]
    fn chunked_with_invalid_trailers() {
        let encoder = Encoder::chunked();

        let trailers = vec![
            AUTHORIZATION,
            CACHE_CONTROL,
            CONTENT_ENCODING,
            CONTENT_LENGTH,
            CONTENT_RANGE,
            CONTENT_TYPE,
            HOST,
            MAX_FORWARDS,
            SET_COOKIE,
            TRAILER,
            TRANSFER_ENCODING,
            TE,
        ];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("header data"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("header data"));
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static("header data"));
        headers.insert(CONTENT_LENGTH, HeaderValue::from_static("header data"));
        headers.insert(CONTENT_RANGE, HeaderValue::from_static("header data"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("header data"));
        headers.insert(HOST, HeaderValue::from_static("header data"));
        headers.insert(MAX_FORWARDS, HeaderValue::from_static("header data"));
        headers.insert(SET_COOKIE, HeaderValue::from_static("header data"));
        headers.insert(TRAILER, HeaderValue::from_static("header data"));
        headers.insert(TRANSFER_ENCODING, HeaderValue::from_static("header data"));
        headers.insert(TE, HeaderValue::from_static("header data"));

        assert!(encoder.encode_trailers::<&[u8]>(headers, true).is_none());
    }

    #[test]
    fn chunked_with_title_case_headers() {
        let encoder = Encoder::chunked();
        let trailers = vec![HeaderName::from_static("chunky-trailer")];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        let headers = HeaderMap::from_iter(vec![(
            HeaderName::from_static("chunky-trailer"),
            HeaderValue::from_static("header data"),
        )]);
        let buf1 = encoder.encode_trailers::<&[u8]>(headers, true).unwrap();

        let mut dst = Vec::new();
        dst.put(buf1);
        assert_eq!(dst, b"0\r\nChunky-Trailer: header data\r\n\r\n");
    }

    #[test]
    fn chunked_trailers_case_insensitive_matching() {
        // Regression test for issue #4010: HTTP/1.1 trailers are case-sensitive
        //
        // Previously, the Trailer header values were stored as HeaderValue (preserving case)
        // and compared against HeaderName (which is always lowercase). This caused trailers
        // declared as "Chunky-Trailer" to not match actual trailers sent as "chunky-trailer".
        //
        // The fix converts Trailer header values to HeaderName during parsing, which
        // normalizes the case and enables proper case-insensitive matching.
        //
        // Note: HeaderName::from_static() requires lowercase input. In real usage,
        // HeaderName::from_bytes() is used to parse the Trailer header value, which
        // normalizes mixed-case input like "Chunky-Trailer" to "chunky-trailer".
        let encoder = Encoder::chunked();
        let trailers = vec![HeaderName::from_static("chunky-trailer")];
        let encoder = encoder.into_chunked_with_trailing_fields(trailers);

        // The actual trailer being sent
        let headers = HeaderMap::from_iter(vec![(
            HeaderName::from_static("chunky-trailer"),
            HeaderValue::from_static("trailer value"),
        )]);

        let buf = encoder.encode_trailers::<&[u8]>(headers, false).unwrap();
        let mut dst = Vec::new();
        dst.put(buf);
        assert_eq!(dst, b"0\r\nchunky-trailer: trailer value\r\n\r\n");
    }
}
