#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use super::*;

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_request() {
        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from("GET /echo HTTP/1.1\r\nHost: hyper.rs\r\n\r\n");
        let mut method = None;
        let msg = Server::parse(
            &mut raw,
            ParseContext {
                cached_headers: &mut None,
                req_method: &mut method,
                h1_parser_config: Default::default(),
                h1_max_headers: None,
                h1_max_header_size: None,
                preserve_header_case: false,
                #[cfg(feature = "ffi")]
                preserve_header_order: false,
                h09_responses: false,
                #[cfg(feature = "client")]
                on_informational: &mut None,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(raw.len(), 0);
        assert_eq!(msg.head.subject.0, crate::Method::GET);
        assert_eq!(msg.head.subject.1, "/echo");
        assert_eq!(msg.head.version, crate::Version::HTTP_11);
        assert_eq!(msg.head.headers.len(), 1);
        assert_eq!(msg.head.headers["Host"], "hyper.rs");
        assert_eq!(method, Some(crate::Method::GET));
    }

    #[test]
    fn test_parse_response() {
        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut Some(crate::Method::GET),
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        let msg = Client::parse(&mut raw, ctx).unwrap().unwrap();
        assert_eq!(raw.len(), 0);
        assert_eq!(msg.head.subject, crate::StatusCode::OK);
        assert_eq!(msg.head.version, crate::Version::HTTP_11);
        assert_eq!(msg.head.headers.len(), 1);
        assert_eq!(msg.head.headers["Content-Length"], "0");
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_request_errors() {
        let mut raw = BytesMut::from("GET htt:p// HTTP/1.1\r\nHost: hyper.rs\r\n\r\n");
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut None,
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        Server::parse(&mut raw, ctx).unwrap_err();
    }

    const H09_RESPONSE: &str = "Baguettes are super delicious, don't you agree?";

    #[test]
    fn test_parse_response_h09_allowed() {
        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from(H09_RESPONSE);
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut Some(crate::Method::GET),
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: true,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        let msg = Client::parse(&mut raw, ctx).unwrap().unwrap();
        assert_eq!(raw, H09_RESPONSE);
        assert_eq!(msg.head.subject, crate::StatusCode::OK);
        assert_eq!(msg.head.version, crate::Version::HTTP_09);
        assert_eq!(msg.head.headers.len(), 0);
    }

    #[test]
    fn test_parse_response_h09_rejected() {
        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from(H09_RESPONSE);
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut Some(crate::Method::GET),
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        Client::parse(&mut raw, ctx).unwrap_err();
        assert_eq!(raw, H09_RESPONSE);
    }

    const RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON: &str =
        "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials : true\r\n\r\n";

    #[test]
    fn test_parse_allow_response_with_spaces_before_colons() {
        use httparse::ParserConfig;

        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from(RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);
        let mut h1_parser_config = ParserConfig::default();
        h1_parser_config.allow_spaces_after_header_name_in_responses(true);
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut Some(crate::Method::GET),
            h1_parser_config,
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        let msg = Client::parse(&mut raw, ctx).unwrap().unwrap();
        assert_eq!(raw.len(), 0);
        assert_eq!(msg.head.subject, crate::StatusCode::OK);
        assert_eq!(msg.head.version, crate::Version::HTTP_11);
        assert_eq!(msg.head.headers.len(), 1);
        assert_eq!(msg.head.headers["Access-Control-Allow-Credentials"], "true");
    }

    #[test]
    fn test_parse_reject_response_with_spaces_before_colons() {
        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from(RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut Some(crate::Method::GET),
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        Client::parse(&mut raw, ctx).unwrap_err();
    }

    const REQUEST_WITH_MULTIPLE_SPACES_IN_REQUEST_LINE: &str =
        "GET  /echo  HTTP/1.1\r\nHost: hyper.rs\r\n\r\n";

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_allow_request_with_multiple_spaces_in_request_line() {
        use httparse::ParserConfig;

        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from(REQUEST_WITH_MULTIPLE_SPACES_IN_REQUEST_LINE);
        let mut h1_parser_config = ParserConfig::default();
        h1_parser_config.allow_multiple_spaces_in_request_line_delimiters(true);
        let mut method = None;
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut method,
            h1_parser_config,
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        let msg = Server::parse(&mut raw, ctx).unwrap().unwrap();
        assert_eq!(raw.len(), 0);
        assert_eq!(msg.head.subject.0, crate::Method::GET);
        assert_eq!(msg.head.subject.1, "/echo");
        assert_eq!(msg.head.version, crate::Version::HTTP_11);
        assert_eq!(msg.head.headers.len(), 1);
        assert_eq!(msg.head.headers["Host"], "hyper.rs");
        assert_eq!(method, Some(crate::Method::GET));
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_reject_request_with_multiple_spaces_in_request_line() {
        let _ = pretty_env_logger::try_init();
        let mut raw = BytesMut::from(REQUEST_WITH_MULTIPLE_SPACES_IN_REQUEST_LINE);
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut None,
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: false,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        Server::parse(&mut raw, ctx).unwrap_err();
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_preserve_header_case_in_request() {
        let mut raw =
            BytesMut::from("GET / HTTP/1.1\r\nHost: hyper.rs\r\nX-BREAD: baguette\r\n\r\n");
        let ctx = ParseContext {
            cached_headers: &mut None,
            req_method: &mut None,
            h1_parser_config: Default::default(),
            h1_max_headers: None,
            h1_max_header_size: None,
            preserve_header_case: true,
            #[cfg(feature = "ffi")]
            preserve_header_order: false,
            h09_responses: false,
            #[cfg(feature = "client")]
            on_informational: &mut None,
        };
        let parsed_message = Server::parse(&mut raw, ctx).unwrap().unwrap();
        let orig_headers = parsed_message
            .head
            .extensions
            .get::<HeaderCaseMap>()
            .unwrap();
        assert_eq!(
            orig_headers
                .get_all_internal(&HeaderName::from_static("host"))
                .collect::<Vec<_>>(),
            vec![&Bytes::from("Host")]
        );
        assert_eq!(
            orig_headers
                .get_all_internal(&HeaderName::from_static("x-bread"))
                .collect::<Vec<_>>(),
            vec![&Bytes::from("X-BREAD")]
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_decoder_request() {
        fn parse(s: &str) -> ParsedMessage<RequestLine> {
            let mut bytes = BytesMut::from(s);
            Server::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .expect("parse ok")
            .expect("parse complete")
        }

        fn parse_err(s: &str, comment: &str) -> crate::error::Parse {
            let mut bytes = BytesMut::from(s);
            Server::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .expect_err(comment)
        }

        // no length or transfer-encoding means 0-length body
        assert_eq!(
            parse(
                "\
                 GET / HTTP/1.1\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::ZERO
        );

        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.1\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::ZERO
        );

        // transfer-encoding: chunked
        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.1\r\n\
                 transfer-encoding: chunked\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CHUNKED
        );

        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.1\r\n\
                 transfer-encoding: gzip, chunked\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CHUNKED
        );

        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.1\r\n\
                 transfer-encoding: gzip\r\n\
                 transfer-encoding: chunked\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CHUNKED
        );

        // content-length
        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.1\r\n\
                 content-length: 10\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::new(10)
        );

        // transfer-encoding and content-length = chunked
        let msg = parse(
            "\
             POST / HTTP/1.1\r\n\
             content-length: 10\r\n\
             transfer-encoding: chunked\r\n\
             \r\n\
             ",
        );
        assert_eq!(msg.decode, DecodedLength::CHUNKED);
        assert!(!msg.head.headers.contains_key(header::CONTENT_LENGTH));
        assert!(!msg.keep_alive);

        let msg = parse(
            "\
             POST / HTTP/1.1\r\n\
             transfer-encoding: chunked\r\n\
             content-length: 10\r\n\
             \r\n\
             ",
        );
        assert_eq!(msg.decode, DecodedLength::CHUNKED);
        assert!(!msg.head.headers.contains_key(header::CONTENT_LENGTH));
        assert!(!msg.keep_alive);

        let msg = parse(
            "\
             POST / HTTP/1.1\r\n\
             transfer-encoding: gzip\r\n\
             content-length: 10\r\n\
             transfer-encoding: chunked\r\n\
             \r\n\
             ",
        );
        assert_eq!(msg.decode, DecodedLength::CHUNKED);
        assert!(!msg.head.headers.contains_key(header::CONTENT_LENGTH));
        assert!(!msg.keep_alive);

        let msg = parse(
            "\
             POST / HTTP/1.1\r\n\
             connection: keep-alive\r\n\
             content-length: 10\r\n\
             transfer-encoding: chunked\r\n\
             \r\n\
             ",
        );
        assert_eq!(msg.decode, DecodedLength::CHUNKED);
        assert!(!msg.head.headers.contains_key(header::CONTENT_LENGTH));
        assert!(!msg.keep_alive);

        // multiple content-lengths of same value are fine
        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.1\r\n\
                 content-length: 10\r\n\
                 content-length: 10\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::new(10)
        );

        // multiple content-lengths with different values is an error
        parse_err(
            "\
             POST / HTTP/1.1\r\n\
             content-length: 10\r\n\
             content-length: 11\r\n\
             \r\n\
             ",
            "multiple content-lengths",
        );

        // content-length with prefix is not allowed
        parse_err(
            "\
             POST / HTTP/1.1\r\n\
             content-length: +10\r\n\
             \r\n\
             ",
            "prefixed content-length",
        );

        // transfer-encoding that isn't chunked is an error
        parse_err(
            "\
             POST / HTTP/1.1\r\n\
             transfer-encoding: gzip\r\n\
             \r\n\
             ",
            "transfer-encoding but not chunked",
        );

        parse_err(
            "\
             POST / HTTP/1.1\r\n\
             transfer-encoding: chunked, gzip\r\n\
             \r\n\
             ",
            "transfer-encoding doesn't end in chunked",
        );

        parse_err(
            "\
             POST / HTTP/1.1\r\n\
             transfer-encoding: chunked\r\n\
             transfer-encoding: afterlol\r\n\
             \r\n\
             ",
            "transfer-encoding multiple lines doesn't end in chunked",
        );

        // http/1.0

        assert_eq!(
            parse(
                "\
                 POST / HTTP/1.0\r\n\
                 content-length: 10\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::new(10)
        );

        // 1.0 doesn't understand chunked, so its an error
        parse_err(
            "\
             POST / HTTP/1.0\r\n\
             transfer-encoding: chunked\r\n\
             \r\n\
             ",
            "1.0 chunked",
        );
    }

    #[test]
    fn test_decoder_response() {
        fn parse(s: &str) -> ParsedMessage<StatusCode> {
            parse_with_method(s, Method::GET)
        }

        fn parse_ignores(s: &str) {
            let mut bytes = BytesMut::from(s);
            assert!(Client::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut Some(Method::GET),
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                }
            )
            .expect("parse ok")
            .is_none())
        }

        fn parse_with_method(s: &str, m: Method) -> ParsedMessage<StatusCode> {
            let mut bytes = BytesMut::from(s);
            Client::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut Some(m),
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .expect("parse ok")
            .expect("parse complete")
        }

        fn parse_err(s: &str) -> crate::error::Parse {
            let mut bytes = BytesMut::from(s);
            Client::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut Some(Method::GET),
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .expect_err("parse should err")
        }

        // no content-length or transfer-encoding means close-delimited
        assert_eq!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CLOSE_DELIMITED
        );

        // 204 and 304 never have a body
        assert_eq!(
            parse(
                "\
                 HTTP/1.1 204 No Content\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::ZERO
        );

        assert_eq!(
            parse(
                "\
                 HTTP/1.1 304 Not Modified\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::ZERO
        );

        // content-length
        assert_eq!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 8\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::new(8)
        );

        assert_eq!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 8\r\n\
                 content-length: 8\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::new(8)
        );

        parse_err(
            "\
             HTTP/1.1 200 OK\r\n\
             content-length: 8\r\n\
             content-length: 9\r\n\
             \r\n\
             ",
        );

        parse_err(
            "\
             HTTP/1.1 200 OK\r\n\
             content-length: +8\r\n\
             \r\n\
             ",
        );

        // transfer-encoding: chunked
        assert_eq!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 transfer-encoding: chunked\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CHUNKED
        );

        // transfer-encoding not-chunked is close-delimited
        assert_eq!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 transfer-encoding: yolo\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CLOSE_DELIMITED
        );

        // transfer-encoding and content-length = chunked
        assert_eq!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 10\r\n\
                 transfer-encoding: chunked\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CHUNKED
        );

        // HEAD can have content-length, but not body
        assert_eq!(
            parse_with_method(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 8\r\n\
                 \r\n\
                 ",
                Method::HEAD
            )
            .decode,
            DecodedLength::ZERO
        );

        // CONNECT with 200 never has body
        {
            let msg = parse_with_method(
                "\
                 HTTP/1.1 200 OK\r\n\
                 \r\n\
                 ",
                Method::CONNECT,
            );
            assert_eq!(msg.decode, DecodedLength::ZERO);
            assert!(!msg.keep_alive, "should be upgrade");
            assert!(msg.wants_upgrade, "should be upgrade");
        }

        // CONNECT receiving non 200 can have a body
        assert_eq!(
            parse_with_method(
                "\
                 HTTP/1.1 400 Bad Request\r\n\
                 \r\n\
                 ",
                Method::CONNECT
            )
            .decode,
            DecodedLength::CLOSE_DELIMITED
        );

        // 1xx status codes
        parse_ignores(
            "\
             HTTP/1.1 100 Continue\r\n\
             \r\n\
             ",
        );

        parse_ignores(
            "\
             HTTP/1.1 103 Early Hints\r\n\
             \r\n\
             ",
        );

        // 101 upgrade not supported yet
        {
            let msg = parse(
                "\
                 HTTP/1.1 101 Switching Protocols\r\n\
                 \r\n\
                 ",
            );
            assert_eq!(msg.decode, DecodedLength::ZERO);
            assert!(!msg.keep_alive, "should be last");
            assert!(msg.wants_upgrade, "should be upgrade");
        }

        // http/1.0
        assert_eq!(
            parse(
                "\
                 HTTP/1.0 200 OK\r\n\
                 \r\n\
                 "
            )
            .decode,
            DecodedLength::CLOSE_DELIMITED
        );

        // 1.0 doesn't understand chunked
        parse_err(
            "\
             HTTP/1.0 200 OK\r\n\
             transfer-encoding: chunked\r\n\
             \r\n\
             ",
        );

        // keep-alive
        assert!(
            parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 0\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "HTTP/1.1 keep-alive is default"
        );

        assert!(
            !parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 0\r\n\
                 connection: foo, close, bar\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "connection close is always close"
        );

        assert!(
            !parse(
                "\
                 HTTP/1.0 200 OK\r\n\
                 content-length: 0\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "HTTP/1.0 close is default"
        );

        assert!(
            parse(
                "\
                 HTTP/1.0 200 OK\r\n\
                 content-length: 0\r\n\
                 connection: foo, keep-alive, bar\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "connection keep-alive is always keep-alive"
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_request_multiple_connection_headers() {
        fn parse(s: &str) -> ParsedMessage<RequestLine> {
            let mut bytes = BytesMut::from(s);
            Server::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .unwrap()
            .unwrap()
        }

        assert!(
            !parse(
                "\
                 GET / HTTP/1.1\r\n\
                 connection: close\r\n\
                 connection: keep-alive\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "close before keep-alive is still close"
        );

        assert!(
            !parse(
                "\
                 GET / HTTP/1.1\r\n\
                 connection: keep-alive\r\n\
                 connection: close\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "keep-alive before close is still close"
        );

        assert!(
            parse(
                "\
                 GET / HTTP/1.0\r\n\
                 connection: keep-alive\r\n\
                 connection: foo\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "HTTP/1.0 keep-alive is not undone by an unrelated token"
        );

        assert!(
            !parse(
                "\
                 GET / HTTP/1.0\r\n\
                 connection: keep-alive\r\n\
                 connection: close\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "HTTP/1.0 close wins over an earlier keep-alive"
        );
    }

    #[cfg(feature = "client")]
    #[test]
    fn test_parse_response_multiple_connection_headers() {
        fn parse(s: &str) -> ParsedMessage<StatusCode> {
            let mut bytes = BytesMut::from(s);
            Client::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut Some(Method::GET),
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .unwrap()
            .unwrap()
        }

        assert!(
            !parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 0\r\n\
                 connection: close\r\n\
                 connection: keep-alive\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "close before keep-alive is still close"
        );

        assert!(
            !parse(
                "\
                 HTTP/1.1 200 OK\r\n\
                 content-length: 0\r\n\
                 connection: keep-alive\r\n\
                 connection: close\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "keep-alive before close is still close"
        );

        assert!(
            parse(
                "\
                 HTTP/1.0 200 OK\r\n\
                 content-length: 0\r\n\
                 connection: keep-alive\r\n\
                 connection: foo\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "HTTP/1.0 keep-alive is not undone by an unrelated token"
        );

        assert!(
            !parse(
                "\
                 HTTP/1.0 200 OK\r\n\
                 content-length: 0\r\n\
                 connection: keep-alive\r\n\
                 connection: close\r\n\
                 \r\n\
                 "
            )
            .keep_alive,
            "HTTP/1.0 close wins over an earlier keep-alive"
        );
    }

    #[cfg(feature = "client")]
    #[test]
    fn test_client_obs_fold_line() {
        fn unfold(src: &str) -> String {
            let mut buf = src.as_bytes().to_vec();
            let mut idx = HeaderIndices {
                name: (0, 0),
                value: (0, buf.len()),
            };
            Client::obs_fold_line(&mut buf, &mut idx);
            String::from_utf8(buf[idx.value.0..idx.value.1].to_vec()).unwrap()
        }

        assert_eq!(unfold("a normal line"), "a normal line",);

        assert_eq!(unfold("obs\r\n fold\r\n\t line"), "obs fold line",);
    }

    #[test]
    fn test_client_request_encode_explicit_transfer_encoding_removes_content_length() {
        use crate::proto::BodyLength;
        use http::header::HeaderValue;

        for values in [
            vec!["10"],
            vec![""],
            vec!["abc"],
            vec!["+10"],
            vec!["18446744073709551616"],
            vec!["10", "11"],
            vec!["10, 11"],
            vec!["10", "abc"],
        ] {
            for transfer_encoding in ["chunked", "gzip"] {
                for body in [BodyLength::Known(10), BodyLength::Unknown] {
                    let mut head = MessageHead::default();
                    for value in &values {
                        head.headers
                            .append("content-length", HeaderValue::from_static(value));
                    }
                    head.headers.insert(
                        "transfer-encoding",
                        HeaderValue::from_static(transfer_encoding),
                    );

                    let mut vec = Vec::new();
                    let encoder = Client::encode(
                        Encode {
                            head: &mut head,
                            body: Some(body),
                            #[cfg(feature = "server")]
                            keep_alive: true,
                            req_method: &mut None,
                            title_case_headers: false,
                            #[cfg(feature = "server")]
                            date_header: true,
                        },
                        &mut vec,
                    )
                    .unwrap();

                    assert!(encoder.is_chunked());
                    let expected_te = if transfer_encoding == "chunked" {
                        "chunked"
                    } else {
                        "gzip, chunked"
                    };
                    assert_eq!(
                        vec,
                        format!("GET / HTTP/1.1\r\ntransfer-encoding: {expected_te}\r\n\r\n")
                            .as_bytes(),
                        "content-length: {values:?}, transfer-encoding: {transfer_encoding}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_client_request_encode_automatic_transfer_encoding_removes_content_length() {
        use crate::proto::BodyLength;
        use http::header::HeaderValue;

        for values in [
            vec![],
            vec![""],
            vec!["abc"],
            vec!["+10"],
            vec!["18446744073709551616"],
            vec!["10", "11"],
            vec!["10, 11"],
            vec!["10", "abc"],
            vec!["10"],
        ] {
            let mut head = RequestHead::default();
            head.subject.0 = Method::POST;
            for value in &values {
                head.headers
                    .append("content-length", HeaderValue::from_static(value));
            }

            let mut vec = Vec::new();
            let encoder = Client::encode(
                Encode {
                    head: &mut head,
                    body: Some(BodyLength::Unknown),
                    #[cfg(feature = "server")]
                    keep_alive: true,
                    req_method: &mut None,
                    title_case_headers: false,
                    #[cfg(feature = "server")]
                    date_header: true,
                },
                &mut vec,
            )
            .unwrap();

            if values == ["10"] {
                assert_eq!(encoder, Encoder::length(10));
                assert_eq!(vec, b"POST / HTTP/1.1\r\ncontent-length: 10\r\n\r\n");
            } else {
                assert!(encoder.is_chunked());
                assert_eq!(
                    vec, b"POST / HTTP/1.1\r\ntransfer-encoding: chunked\r\n\r\n",
                    "content-length: {values:?}"
                );
            }
        }
    }

    #[test]
    fn test_client_request_encode_title_case() {
        use crate::proto::BodyLength;
        use http::header::HeaderValue;

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));
        head.headers.insert("*-*", HeaderValue::from_static("o_o"));

        let mut vec = Vec::new();
        Client::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                #[cfg(feature = "server")]
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: true,
                #[cfg(feature = "server")]
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        assert_eq!(vec, b"GET / HTTP/1.1\r\nContent-Length: 10\r\nContent-Type: application/json\r\n*-*: o_o\r\n\r\n".to_vec());
    }

    #[test]
    fn test_client_request_encode_orig_case() {
        use crate::proto::BodyLength;
        use http::header::{HeaderValue, CONTENT_LENGTH};

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));

        let mut orig_headers = HeaderCaseMap::default();
        orig_headers.insert(CONTENT_LENGTH, "CONTENT-LENGTH".into());
        head.extensions.insert(orig_headers);

        let mut vec = Vec::new();
        Client::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                #[cfg(feature = "server")]
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: false,
                #[cfg(feature = "server")]
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        assert_eq!(
            &*vec,
            b"GET / HTTP/1.1\r\nCONTENT-LENGTH: 10\r\ncontent-type: application/json\r\n\r\n"
                .as_ref(),
        );
    }
    #[test]
    fn test_client_request_encode_orig_and_title_case() {
        use crate::proto::BodyLength;
        use http::header::{HeaderValue, CONTENT_LENGTH};

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));

        let mut orig_headers = HeaderCaseMap::default();
        orig_headers.insert(CONTENT_LENGTH, "CONTENT-LENGTH".into());
        head.extensions.insert(orig_headers);

        let mut vec = Vec::new();
        Client::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                #[cfg(feature = "server")]
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: true,
                #[cfg(feature = "server")]
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        assert_eq!(
            &*vec,
            b"GET / HTTP/1.1\r\nCONTENT-LENGTH: 10\r\nContent-Type: application/json\r\n\r\n"
                .as_ref(),
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_server_encode_connect_method() {
        let mut head = MessageHead::default();

        let mut vec = Vec::new();
        let encoder = Server::encode(
            Encode {
                head: &mut head,
                body: None,
                keep_alive: true,
                req_method: &mut Some(Method::CONNECT),
                title_case_headers: false,
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        assert!(encoder.is_last());
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_server_response_encode_title_case() {
        use crate::proto::BodyLength;
        use http::header::HeaderValue;

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));
        head.headers
            .insert("weird--header", HeaderValue::from_static(""));

        let mut vec = Vec::new();
        Server::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: true,
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        let expected_response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nContent-Type: application/json\r\nWeird--Header: \r\n";

        assert_eq!(&vec[..expected_response.len()], &expected_response[..]);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_server_response_encode_orig_case() {
        use crate::proto::BodyLength;
        use http::header::{HeaderValue, CONTENT_LENGTH};

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));

        let mut orig_headers = HeaderCaseMap::default();
        orig_headers.insert(CONTENT_LENGTH, "CONTENT-LENGTH".into());
        head.extensions.insert(orig_headers);

        let mut vec = Vec::new();
        Server::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: false,
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        let expected_response =
            b"HTTP/1.1 200 OK\r\nCONTENT-LENGTH: 10\r\ncontent-type: application/json\r\ndate: ";

        assert_eq!(&vec[..expected_response.len()], &expected_response[..]);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_server_response_encode_orig_and_title_case() {
        use crate::proto::BodyLength;
        use http::header::{HeaderValue, CONTENT_LENGTH};

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));

        let mut orig_headers = HeaderCaseMap::default();
        orig_headers.insert(CONTENT_LENGTH, "CONTENT-LENGTH".into());
        head.extensions.insert(orig_headers);

        let mut vec = Vec::new();
        Server::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: true,
                date_header: true,
            },
            &mut vec,
        )
        .unwrap();

        // this will also test that the date does exist
        let expected_response =
            b"HTTP/1.1 200 OK\r\nCONTENT-LENGTH: 10\r\nContent-Type: application/json\r\nDate: ";

        assert_eq!(&vec[..expected_response.len()], &expected_response[..]);
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_disabled_date_header() {
        use crate::proto::BodyLength;
        use http::header::{HeaderValue, CONTENT_LENGTH};

        let mut head = MessageHead::default();
        head.headers
            .insert("content-length", HeaderValue::from_static("10"));
        head.headers
            .insert("content-type", HeaderValue::from_static("application/json"));

        let mut orig_headers = HeaderCaseMap::default();
        orig_headers.insert(CONTENT_LENGTH, "CONTENT-LENGTH".into());
        head.extensions.insert(orig_headers);

        let mut vec = Vec::new();
        Server::encode(
            Encode {
                head: &mut head,
                body: Some(BodyLength::Known(10)),
                keep_alive: true,
                req_method: &mut None,
                title_case_headers: true,
                date_header: false,
            },
            &mut vec,
        )
        .unwrap();

        let expected_response =
            b"HTTP/1.1 200 OK\r\nCONTENT-LENGTH: 10\r\nContent-Type: application/json\r\n\r\n";

        assert_eq!(&vec, &expected_response);
    }

    #[test]
    fn parse_header_htabs() {
        let mut bytes = BytesMut::from("HTTP/1.1 200 OK\r\nserver: hello\tworld\r\n\r\n");
        let parsed = Client::parse(
            &mut bytes,
            ParseContext {
                cached_headers: &mut None,
                req_method: &mut Some(Method::GET),
                h1_parser_config: Default::default(),
                h1_max_headers: None,
                h1_max_header_size: None,
                preserve_header_case: false,
                #[cfg(feature = "ffi")]
                preserve_header_order: false,
                h09_responses: false,
                #[cfg(feature = "client")]
                on_informational: &mut None,
            },
        )
        .expect("parse ok")
        .expect("parse complete");

        assert_eq!(parsed.head.headers["server"], "hello\tworld");
    }

    #[cfg(feature = "server")]
    #[test]
    fn parse_too_large_headers() {
        fn gen_req_with_headers(num: usize) -> String {
            let mut req = String::from("GET / HTTP/1.1\r\n");
            for i in 0..num {
                req.push_str(&format!("key{i}: val{i}\r\n"));
            }
            req.push_str("\r\n");
            req
        }
        fn gen_resp_with_headers(num: usize) -> String {
            let mut req = String::from("HTTP/1.1 200 OK\r\n");
            for i in 0..num {
                req.push_str(&format!("key{i}: val{i}\r\n"));
            }
            req.push_str("\r\n");
            req
        }
        fn parse(max_headers: Option<usize>, gen_size: usize, should_success: bool) {
            {
                // server side
                let mut bytes = BytesMut::from(gen_req_with_headers(gen_size).as_str());
                let result = Server::parse(
                    &mut bytes,
                    ParseContext {
                        cached_headers: &mut None,
                        req_method: &mut None,
                        h1_parser_config: Default::default(),
                        h1_max_headers: max_headers,
                        h1_max_header_size: None,
                        preserve_header_case: false,
                        #[cfg(feature = "ffi")]
                        preserve_header_order: false,
                        h09_responses: false,
                        #[cfg(feature = "client")]
                        on_informational: &mut None,
                    },
                );
                if should_success {
                    result.expect("parse ok").expect("parse complete");
                } else {
                    result.expect_err("parse should err");
                }
            }
            {
                // client side
                let mut bytes = BytesMut::from(gen_resp_with_headers(gen_size).as_str());
                let result = Client::parse(
                    &mut bytes,
                    ParseContext {
                        cached_headers: &mut None,
                        req_method: &mut None,
                        h1_parser_config: Default::default(),
                        h1_max_headers: max_headers,
                        h1_max_header_size: None,
                        preserve_header_case: false,
                        #[cfg(feature = "ffi")]
                        preserve_header_order: false,
                        h09_responses: false,
                        #[cfg(feature = "client")]
                        on_informational: &mut None,
                    },
                );
                if should_success {
                    result.expect("parse ok").expect("parse complete");
                } else {
                    result.expect_err("parse should err");
                }
            }
        }

        // check generator
        assert_eq!(
            gen_req_with_headers(0),
            String::from("GET / HTTP/1.1\r\n\r\n")
        );
        assert_eq!(
            gen_req_with_headers(1),
            String::from("GET / HTTP/1.1\r\nkey0: val0\r\n\r\n")
        );
        assert_eq!(
            gen_req_with_headers(2),
            String::from("GET / HTTP/1.1\r\nkey0: val0\r\nkey1: val1\r\n\r\n")
        );
        assert_eq!(
            gen_req_with_headers(3),
            String::from("GET / HTTP/1.1\r\nkey0: val0\r\nkey1: val1\r\nkey2: val2\r\n\r\n")
        );

        // default max_headers is 100, so
        //
        // - less than or equal to 100, accepted
        //
        parse(None, 0, true);
        parse(None, 1, true);
        parse(None, 50, true);
        parse(None, 99, true);
        parse(None, 100, true);
        //
        // - more than 100, rejected
        //
        parse(None, 101, false);
        parse(None, 102, false);
        parse(None, 200, false);

        // max_headers is 0, parser will reject any headers
        //
        // - without header, accepted
        //
        parse(Some(0), 0, true);
        //
        // - with header(s), rejected
        //
        parse(Some(0), 1, false);
        parse(Some(0), 100, false);

        // max_headers is 200
        //
        // - less than or equal to 200, accepted
        //
        parse(Some(200), 0, true);
        parse(Some(200), 1, true);
        parse(Some(200), 100, true);
        parse(Some(200), 200, true);
        //
        // - more than 200, rejected
        //
        parse(Some(200), 201, false);
        parse(Some(200), 210, false);
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_h1_server_max_header_size() {
        let _ = pretty_env_logger::try_init();

        let req_str = "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        assert_eq!(req_str.len(), 37);

        let parse_req = |max_header_size: Option<usize>| {
            let mut bytes = BytesMut::from(req_str);
            Server::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: max_header_size,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
        };

        // Server checks
        parse_req(None).unwrap().unwrap();
        parse_req(Some(37)).unwrap().unwrap();
        parse_req(Some(50)).unwrap().unwrap();
        assert!(matches!(parse_req(Some(36)), Err(Parse::TooLarge)));
        assert!(matches!(parse_req(Some(10)), Err(Parse::TooLarge)));
    }

    #[test]
    #[cfg(feature = "client")]
    fn test_h1_client_max_header_size() {
        let _ = pretty_env_logger::try_init();

        let resp_str = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(resp_str.len(), 38);

        let parse_resp = |max_header_size: Option<usize>| {
            let mut bytes = BytesMut::from(resp_str);
            Client::parse(
                &mut bytes,
                ParseContext {
                    cached_headers: &mut None,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: max_header_size,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
        };

        // Client checks
        parse_resp(None).unwrap().unwrap();
        parse_resp(Some(38)).unwrap().unwrap();
        parse_resp(Some(50)).unwrap().unwrap();
        assert!(matches!(parse_resp(Some(37)), Err(Parse::TooLarge)));
        assert!(matches!(parse_resp(Some(10)), Err(Parse::TooLarge)));
    }

    #[test]
    fn test_is_complete_fast() {
        let s = b"GET / HTTP/1.1\r\na: b\r\n\r\n";
        for n in 0..s.len() {
            assert!(is_complete_fast(s, n), "{:?}; {}", s, n);
        }
        let s = b"GET / HTTP/1.1\na: b\n\n";
        for n in 0..s.len() {
            assert!(is_complete_fast(s, n));
        }
        let s = b"GET / HTTP/1.1\r\na: b\n\r\n";
        for n in 0..s.len() {
            assert!(is_complete_fast(s, n), "{:?}; {}", s, n);
        }

        // Not
        let s = b"GET / HTTP/1.1\r\na: b\r\n\r";
        for n in 0..s.len() {
            assert!(!is_complete_fast(s, n));
        }
        let s = b"GET / HTTP/1.1\na: b\n";
        for n in 0..s.len() {
            assert!(!is_complete_fast(s, n));
        }
        let s = b"GET / HTTP/1.1\r\na: b\n\r";
        for n in 0..s.len() {
            assert!(!is_complete_fast(s, n));
        }
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_parse_accepts_lf_crlf_terminator() {
        // The full parser (httparse) accepts a bare-LF line ending followed
        // by a CRLF blank line as the end of the head, so the partial-read
        // fast path must recognize it too.
        let mut bytes = BytesMut::from("GET / HTTP/1.1\r\na: b\n\r\n");
        Server::parse(
            &mut bytes,
            ParseContext {
                cached_headers: &mut None,
                req_method: &mut None,
                h1_parser_config: Default::default(),
                h1_max_headers: None,
                h1_max_header_size: None,
                preserve_header_case: false,
                #[cfg(feature = "ffi")]
                preserve_header_order: false,
                h09_responses: false,
                #[cfg(feature = "client")]
                on_informational: &mut None,
            },
        )
        .expect("parse ok")
        .expect("parse complete");
    }

    #[test]
    fn test_write_headers_orig_case_empty_value() {
        let mut headers = HeaderMap::new();
        let name = http::header::HeaderName::from_static("x-empty");
        headers.insert(&name, "".parse().expect("parse empty"));
        let mut orig_cases = HeaderCaseMap::default();
        orig_cases.insert(name, Bytes::from_static(b"X-EmptY"));

        let mut dst = Vec::new();
        super::write_headers_original_case(&headers, &orig_cases, &mut dst, false);

        assert_eq!(
            dst, b"X-EmptY:\r\n",
            "there should be no space between the colon and CRLF"
        );
    }

    #[test]
    fn test_write_headers_orig_case_multiple_entries() {
        let mut headers = HeaderMap::new();
        let name = http::header::HeaderName::from_static("x-empty");
        headers.insert(&name, "a".parse().unwrap());
        headers.append(&name, "b".parse().unwrap());

        let mut orig_cases = HeaderCaseMap::default();
        orig_cases.insert(name.clone(), Bytes::from_static(b"X-Empty"));
        orig_cases.append(name, Bytes::from_static(b"X-EMPTY"));

        let mut dst = Vec::new();
        super::write_headers_original_case(&headers, &orig_cases, &mut dst, false);

        assert_eq!(dst, b"X-Empty: a\r\nX-EMPTY: b\r\n");
    }

    #[cfg(feature = "nightly")]
    use test::Bencher;

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_parse_incoming(b: &mut Bencher) {
        let mut raw = BytesMut::from(
            &b"GET /super_long_uri/and_whatever?what_should_we_talk_about/\
            I_wonder/Hard_to_write_in_an_uri_after_all/you_have_to_make\
            _up_the_punctuation_yourself/how_fun_is_that?test=foo&test1=\
            foo1&test2=foo2&test3=foo3&test4=foo4 HTTP/1.1\r\nHost: \
            hyper.rs\r\nAccept: a lot of things\r\nAccept-Charset: \
            utf8\r\nAccept-Encoding: *\r\nAccess-Control-Allow-\
            Credentials: None\r\nAccess-Control-Allow-Origin: None\r\n\
            Access-Control-Allow-Methods: None\r\nAccess-Control-Allow-\
            Headers: None\r\nContent-Encoding: utf8\r\nContent-Security-\
            Policy: None\r\nContent-Type: text/html\r\nOrigin: hyper\
            \r\nSec-Websocket-Extensions: It looks super important!\r\n\
            Sec-Websocket-Origin: hyper\r\nSec-Websocket-Version: 4.3\r\
            \nStrict-Transport-Security: None\r\nUser-Agent: hyper\r\n\
            X-Content-Duration: None\r\nX-Content-Security-Policy: None\
            \r\nX-DNSPrefetch-Control: None\r\nX-Frame-Options: \
            Something important obviously\r\nX-Requested-With: Nothing\
            \r\n\r\n"[..],
        );
        let len = raw.len();
        let mut headers = Some(HeaderMap::new());

        b.bytes = len as u64;
        b.iter(|| {
            let mut msg = Server::parse(
                &mut raw,
                ParseContext {
                    cached_headers: &mut headers,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .unwrap()
            .unwrap();
            ::test::black_box(&msg);

            // Remove all references pointing into BytesMut.
            msg.head.headers.clear();
            headers = Some(msg.head.headers);
            std::mem::take(&mut msg.head.subject);

            restart(&mut raw, len);
        });

        fn restart(b: &mut BytesMut, len: usize) {
            b.reserve(1);
            unsafe {
                b.set_len(len);
            }
        }
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_parse_short(b: &mut Bencher) {
        let s = &b"GET / HTTP/1.1\r\nHost: localhost:8080\r\n\r\n"[..];
        let mut raw = BytesMut::from(s);
        let len = raw.len();
        let mut headers = Some(HeaderMap::new());

        b.bytes = len as u64;
        b.iter(|| {
            let mut msg = Server::parse(
                &mut raw,
                ParseContext {
                    cached_headers: &mut headers,
                    req_method: &mut None,
                    h1_parser_config: Default::default(),
                    h1_max_headers: None,
                    h1_max_header_size: None,
                    preserve_header_case: false,
                    #[cfg(feature = "ffi")]
                    preserve_header_order: false,
                    h09_responses: false,
                    #[cfg(feature = "client")]
                    on_informational: &mut None,
                },
            )
            .unwrap()
            .unwrap();
            ::test::black_box(&msg);
            msg.head.headers.clear();
            headers = Some(msg.head.headers);
            restart(&mut raw, len);
        });

        fn restart(b: &mut BytesMut, len: usize) {
            b.reserve(1);
            unsafe {
                b.set_len(len);
            }
        }
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_server_encode_headers_preset(b: &mut Bencher) {
        use crate::proto::BodyLength;
        use http::header::HeaderValue;

        let len = 108;
        b.bytes = len as u64;

        let mut head = MessageHead::default();
        let mut headers = HeaderMap::new();
        headers.insert("content-length", HeaderValue::from_static("10"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        b.iter(|| {
            let mut vec = Vec::new();
            head.headers = headers.clone();
            Server::encode(
                Encode {
                    head: &mut head,
                    body: Some(BodyLength::Known(10)),
                    keep_alive: true,
                    req_method: &mut Some(Method::GET),
                    title_case_headers: false,
                    date_header: true,
                },
                &mut vec,
            )
            .unwrap();
            assert_eq!(vec.len(), len);
            ::test::black_box(vec);
        })
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_server_encode_no_headers(b: &mut Bencher) {
        use crate::proto::BodyLength;

        let len = 76;
        b.bytes = len as u64;

        let mut head = MessageHead::default();
        let mut vec = Vec::with_capacity(128);

        b.iter(|| {
            Server::encode(
                Encode {
                    head: &mut head,
                    body: Some(BodyLength::Known(10)),
                    keep_alive: true,
                    req_method: &mut Some(Method::GET),
                    title_case_headers: false,
                    date_header: true,
                },
                &mut vec,
            )
            .unwrap();
            assert_eq!(vec.len(), len);
            ::test::black_box(&vec);

            vec.clear();
        })
    }
}
