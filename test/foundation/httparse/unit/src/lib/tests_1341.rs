#[cfg(test)]
mod tests {
    use super::{Error, Request, Response, Status, EMPTY_HEADER, parse_chunk_size};

    const NUM_OF_HEADERS: usize = 4;

    macro_rules! req {
        ($name:ident, $buf:expr, |$arg:ident| $body:expr) => (
            req! {$name, $buf, Ok(Status::Complete($buf.len())), |$arg| $body }
        );
        ($name:ident, $buf:expr, $len:expr, |$arg:ident| $body:expr) => (
        #[test]
        fn $name() {
            let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
            let mut req = Request::new(&mut headers[..]);
            let status = req.parse($buf.as_ref());
            assert_eq!(status, $len);
            closure(req);

            fn closure($arg: Request) {
                $body
            }
        }
        )
    }

    req! {
        test_request_simple,
        b"GET / HTTP/1.1\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 0);
        }
    }

    req! {
        test_request_simple_with_query_params,
        b"GET /thing?data=a HTTP/1.1\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/thing?data=a");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 0);
        }
    }

    req! {
        test_request_simple_with_whatwg_query_params,
        b"GET /thing?data=a^ HTTP/1.1\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/thing?data=a^");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 0);
        }
    }

    req! {
        test_request_headers,
        b"GET / HTTP/1.1\r\nHost: foo.com\r\nCookie: \r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 2);
            assert_eq!(req.headers[0].name, "Host");
            assert_eq!(req.headers[0].value, b"foo.com");
            assert_eq!(req.headers[1].name, "Cookie");
            assert_eq!(req.headers[1].value, b"");
        }
    }

    req! {
        test_request_headers_optional_whitespace,
        b"GET / HTTP/1.1\r\nHost: \tfoo.com\t \r\nCookie: \t \r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 2);
            assert_eq!(req.headers[0].name, "Host");
            assert_eq!(req.headers[0].value, b"foo.com");
            assert_eq!(req.headers[1].name, "Cookie");
            assert_eq!(req.headers[1].value, b"");
        }
    }

    req! {
        // test the scalar parsing
        test_request_header_value_htab_short,
        b"GET / HTTP/1.1\r\nUser-Agent: some\tagent\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 1);
            assert_eq!(req.headers[0].name, "User-Agent");
            assert_eq!(req.headers[0].value, b"some\tagent");
        }
    }

    req! {
        // test the sse42 parsing
        test_request_header_value_htab_med,
        b"GET / HTTP/1.1\r\nUser-Agent: 1234567890some\tagent\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 1);
            assert_eq!(req.headers[0].name, "User-Agent");
            assert_eq!(req.headers[0].value, b"1234567890some\tagent");
        }
    }

    req! {
        // test the avx2 parsing
        test_request_header_value_htab_long,
        b"GET / HTTP/1.1\r\nUser-Agent: 1234567890some\t1234567890agent1234567890\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 1);
            assert_eq!(req.headers[0].name, "User-Agent");
            assert_eq!(req.headers[0].value, &b"1234567890some\t1234567890agent1234567890"[..]);
        }
    }

    req! {
        // test the avx2 parsing
        test_request_header_no_space_after_colon,
        b"GET / HTTP/1.1\r\nUser-Agent:omg-no-space1234567890some1234567890agent1234567890\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 1);
            assert_eq!(req.headers[0].name, "User-Agent");
            assert_eq!(req.headers[0].value, &b"omg-no-space1234567890some1234567890agent1234567890"[..]);
        }
    }

    req! {
        test_request_headers_max,
        b"GET / HTTP/1.1\r\nA: A\r\nB: B\r\nC: C\r\nD: D\r\n\r\n",
        |req| {
            assert_eq!(req.headers.len(), NUM_OF_HEADERS);
        }
    }

    req! {
        test_request_multibyte,
        b"GET / HTTP/1.1\r\nHost: foo.com\r\nUser-Agent: \xe3\x81\xb2\xe3/1.0\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 2);
            assert_eq!(req.headers[0].name, "Host");
            assert_eq!(req.headers[0].value, b"foo.com");
            assert_eq!(req.headers[1].name, "User-Agent");
            assert_eq!(req.headers[1].value, b"\xe3\x81\xb2\xe3/1.0");
        }
    }

    // A single byte which is part of a method is not invalid
    req! {
        test_request_one_byte_method,
        b"G", Ok(Status::Partial),
        |_req| {}
    }

    // A subset of a method is a partial method, not invalid
    req! {
        test_request_partial_method,
        b"GE", Ok(Status::Partial),
        |_req| {}
    }

    // A method, without the delimiting space, is a partial request
    req! {
        test_request_method_no_delimiter,
        b"GET", Ok(Status::Partial),
        |_req| {}
    }

    // Regression test: assert that a partial read with just the method and
    // space results in a partial, rather than a token error from uri parsing.
    req! {
        test_request_method_only,
        b"GET ", Ok(Status::Partial),
        |_req| {}
    }

    req! {
        test_request_partial,
        b"GET / HTTP/1.1\r\n\r", Ok(Status::Partial),
        |_req| {}
    }

    req! {
        test_request_partial_version,
        b"GET / HTTP/1.", Ok(Status::Partial),
        |_req| {}
    }

    req! {
        test_request_method_path_no_delimiter,
        b"GET /", Ok(Status::Partial),
        |_req| {}
    }

    req! {
        test_request_method_path_only,
        b"GET / ", Ok(Status::Partial),
        |_req| {}
    }

    req! {
        test_request_partial_parses_headers_as_much_as_it_can,
        b"GET / HTTP/1.1\r\nHost: yolo\r\n",
        Ok(crate::Status::Partial),
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), NUM_OF_HEADERS); // doesn't slice since not Complete
            assert_eq!(req.headers[0].name, "Host");
            assert_eq!(req.headers[0].value, b"yolo");
        }
    }

    req! {
        test_request_newlines,
        b"GET / HTTP/1.1\nHost: foo.bar\n\n",
        |_r| {}
    }

    req! {
        test_request_empty_lines_prefix,
        b"\r\n\r\nGET / HTTP/1.1\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 0);
        }
    }

    req! {
        test_request_empty_lines_prefix_lf_only,
        b"\n\nGET / HTTP/1.1\n\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 0);
        }
    }

    req! {
        test_request_path_backslash,
        b"\n\nGET /\\?wayne\\=5 HTTP/1.1\n\n",
        |req| {
            assert_eq!(req.method.unwrap(), "GET");
            assert_eq!(req.path.unwrap(), "/\\?wayne\\=5");
            assert_eq!(req.version.unwrap(), 1);
            assert_eq!(req.headers.len(), 0);
        }
    }

    req! {
        test_request_with_invalid_token_delimiter,
        b"GET\n/ HTTP/1.1\r\nHost: foo.bar\r\n\r\n",
        Err(crate::Error::Token),
        |_r| {}
    }


    req! {
        test_request_with_invalid_but_short_version,
        b"GET / HTTP/1!",
        Err(crate::Error::Version),
        |_r| {}
    }

    req! {
        test_request_with_empty_method,
        b" / HTTP/1.1\r\n\r\n",
        Err(crate::Error::Token),
        |_r| {}
    }

    req! {
        test_request_with_empty_path,
        b"GET  HTTP/1.1\r\n\r\n",
        Err(crate::Error::Token),
        |_r| {}
    }

    req! {
        test_request_with_empty_method_and_path,
        b"  HTTP/1.1\r\n\r\n",
        Err(crate::Error::Token),
        |_r| {}
    }

    macro_rules! res {
        ($name:ident, $buf:expr, |$arg:ident| $body:expr) => (
            res! {$name, $buf, Ok(Status::Complete($buf.len())), |$arg| $body }
        );
        ($name:ident, $buf:expr, $len:expr, |$arg:ident| $body:expr) => (
        #[test]
        fn $name() {
            let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
            let mut res = Response::new(&mut headers[..]);
            let status = res.parse($buf.as_ref());
            assert_eq!(status, $len);
            closure(res);

            fn closure($arg: Response) {
                $body
            }
        }
        )
    }

    res! {
        test_response_simple,
        b"HTTP/1.1 200 OK\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "OK");
        }
    }

    res! {
        test_response_newlines,
        b"HTTP/1.0 403 Forbidden\nServer: foo.bar\n\n",
        |_r| {}
    }

    res! {
        test_response_reason_missing,
        b"HTTP/1.1 200 \r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "");
        }
    }

    res! {
        test_response_reason_missing_no_space,
        b"HTTP/1.1 200\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "");
        }
    }

    res! {
        test_response_reason_missing_no_space_with_headers,
        b"HTTP/1.1 200\r\nFoo: bar\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "");
            assert_eq!(res.headers.len(), 1);
            assert_eq!(res.headers[0].name, "Foo");
            assert_eq!(res.headers[0].value, b"bar");
        }
    }

    res! {
        test_response_reason_with_space_and_tab,
        b"HTTP/1.1 101 Switching Protocols\t\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 101);
            assert_eq!(res.reason.unwrap(), "Switching Protocols\t");
        }
    }

    static RESPONSE_REASON_WITH_OBS_TEXT_BYTE: &[u8] = b"HTTP/1.1 200 X\xFFZ\r\n\r\n";
    res! {
        test_response_reason_with_obsolete_text_byte,
        RESPONSE_REASON_WITH_OBS_TEXT_BYTE,
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 200);
            // Empty string fallback in case of obs-text
            assert_eq!(res.reason.unwrap(), "");
        }
    }

    res! {
        test_response_reason_with_nul_byte,
        b"HTTP/1.1 200 \x00\r\n\r\n",
        Err(crate::Error::Status),
        |_res| {}
    }

    res! {
        test_response_version_missing_space,
        b"HTTP/1.1",
        Ok(Status::Partial),
        |_res| {}
    }

    res! {
        test_response_code_missing_space,
        b"HTTP/1.1 200",
        Ok(Status::Partial),
        |_res| {}
    }

    res! {
        test_response_partial_parses_headers_as_much_as_it_can,
        b"HTTP/1.1 200 OK\r\nServer: yolo\r\n",
        Ok(crate::Status::Partial),
        |res| {
            assert_eq!(res.version.unwrap(), 1);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "OK");
            assert_eq!(res.headers.len(), NUM_OF_HEADERS); // doesn't slice since not Complete
            assert_eq!(res.headers[0].name, "Server");
            assert_eq!(res.headers[0].value, b"yolo");
        }
    }

    res! {
        test_response_empty_lines_prefix_lf_only,
        b"\n\nHTTP/1.1 200 OK\n\n",
        |_res| {}
    }

    res! {
        test_response_no_cr,
        b"HTTP/1.0 200\nContent-type: text/html\n\n",
        |res| {
            assert_eq!(res.version.unwrap(), 0);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "");
            assert_eq!(res.headers.len(), 1);
            assert_eq!(res.headers[0].name, "Content-type");
            assert_eq!(res.headers[0].value, b"text/html");
        }
    }

    /// Check all subset permutations of a partial request line with no headers
    #[test]
    fn partial_permutations() {
        let req_str = "GET / HTTP/1.1\r\n\r\n";
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut req = Request::new(&mut headers[..]);
        for i in 0..req_str.len() {
            let status = req.parse(req_str[..i].as_bytes());
            assert_eq!(
                status,
                Ok(Status::Partial),
                "partial request line should return partial. \
                 Portion which failed: '{seg}' (below {i})",
                seg = &req_str[..i]
            );
        }
    }

    static RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON: &[u8] =
        b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials : true\r\nBread: baguette\r\n\r\n";

    #[test]
    fn test_forbid_response_with_whitespace_between_header_name_and_colon() {
        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_allow_response_with_whitespace_between_header_name_and_colon() {
        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_spaces_after_header_name_in_responses(true)
            .parse_response(&mut response, RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);

        assert_eq!(result, Ok(Status::Complete(77)));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 2);
        assert_eq!(response.headers[0].name, "Access-Control-Allow-Credentials");
        assert_eq!(response.headers[0].value, &b"true"[..]);
        assert_eq!(response.headers[1].name, "Bread");
        assert_eq!(response.headers[1].value, &b"baguette"[..]);
    }

    #[test]
    fn test_ignore_header_line_with_whitespaces_after_header_name_in_response() {
        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);

        assert_eq!(result, Ok(Status::Complete(77)));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);
    }

    static REQUEST_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON: &[u8] =
        b"GET / HTTP/1.1\r\nHost : localhost\r\n\r\n";

    #[test]
    fn test_forbid_request_with_whitespace_between_header_name_and_colon() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut request = Request::new(&mut headers[..]);
        let result = request.parse(REQUEST_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_ignore_header_line_with_whitespaces_after_header_name_in_request() {
        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST_WITH_WHITESPACE_BETWEEN_HEADER_NAME_AND_COLON);

        assert_eq!(result, Ok(Status::Complete(36)));
    }

    static RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_START: &[u8] =
        b"HTTP/1.1 200 OK\r\nLine-Folded-Header: \r\n   \r\n hello there\r\n\r\n";

    #[test]
    fn test_forbid_response_with_obsolete_line_folding_at_start() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_START);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_allow_response_with_obsolete_line_folding_at_start() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_obsolete_multiline_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_START);

        assert_eq!(result, Ok(Status::Complete(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_START.len())));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Line-Folded-Header");
        assert_eq!(response.headers[0].value, &b"hello there"[..]);
    }

    static RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_END: &[u8] =
        b"HTTP/1.1 200 OK\r\nLine-Folded-Header: hello there\r\n   \r\n \r\n\r\n";

    #[test]
    fn test_forbid_response_with_obsolete_line_folding_at_end() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_END);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_allow_response_with_obsolete_line_folding_at_end() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_obsolete_multiline_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_END);

        assert_eq!(result, Ok(Status::Complete(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_AT_END.len())));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Line-Folded-Header");
        assert_eq!(response.headers[0].value, &b"hello there"[..]);
    }

    static RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_MIDDLE: &[u8] =
        b"HTTP/1.1 200 OK\r\nLine-Folded-Header: hello  \r\n \r\n there\r\n\r\n";

    #[test]
    fn test_forbid_response_with_obsolete_line_folding_in_middle() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_MIDDLE);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_allow_response_with_obsolete_line_folding_in_middle() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_obsolete_multiline_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_MIDDLE);

        assert_eq!(result, Ok(Status::Complete(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_MIDDLE.len())));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Line-Folded-Header");
        assert_eq!(response.headers[0].value, &b"hello  \r\n \r\n there"[..]);
    }

    static RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_EMPTY_HEADER: &[u8] =
        b"HTTP/1.1 200 OK\r\nLine-Folded-Header:   \r\n \r\n \r\n\r\n";

    #[test]
    fn test_forbid_response_with_obsolete_line_folding_in_empty_header() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_EMPTY_HEADER);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_allow_response_with_obsolete_line_folding_in_empty_header() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_obsolete_multiline_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_EMPTY_HEADER);

        assert_eq!(result, Ok(Status::Complete(RESPONSE_WITH_OBSOLETE_LINE_FOLDING_IN_EMPTY_HEADER.len())));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Line-Folded-Header");
        assert_eq!(response.headers[0].value, &b""[..]);
    }

    #[test]
    fn test_chunk_size() {
        assert_eq!(parse_chunk_size(b"0\r\n"), Ok(Status::Complete((3, 0))));
        assert_eq!(parse_chunk_size(b"12\r\nchunk"), Ok(Status::Complete((4, 18))));
        assert_eq!(parse_chunk_size(b"3086d\r\n"), Ok(Status::Complete((7, 198765))));
        assert_eq!(parse_chunk_size(b"3735AB1;foo bar*\r\n"), Ok(Status::Complete((18, 57891505))));
        assert_eq!(parse_chunk_size(b"3735ab1 ; baz \r\n"), Ok(Status::Complete((16, 57891505))));
        assert_eq!(parse_chunk_size(b"77a65\r"), Ok(Status::Partial));
        assert_eq!(parse_chunk_size(b"ab"), Ok(Status::Partial));
        assert_eq!(parse_chunk_size(b"567f8a\rfoo"), Err(crate::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"567f8a\rfoo"), Err(crate::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"567xf8a\r\n"), Err(crate::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"ffffffffffffffff\r\n"), Ok(Status::Complete((18, u64::MAX))));
        assert_eq!(parse_chunk_size(b"1ffffffffffffffff\r\n"), Err(crate::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"Affffffffffffffff\r\n"), Err(crate::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"fffffffffffffffff\r\n"), Err(crate::InvalidChunkSize));
    }

    static RESPONSE_WITH_MULTIPLE_SPACE_DELIMITERS: &[u8] =
        b"HTTP/1.1   200  OK\r\n\r\n";

    #[test]
    fn test_forbid_response_with_multiple_space_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_MULTIPLE_SPACE_DELIMITERS);

        assert_eq!(result, Err(crate::Error::Status));
    }

    #[test]
    fn test_allow_response_with_multiple_space_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_response_status_delimiters(true)
            .parse_response(&mut response, RESPONSE_WITH_MULTIPLE_SPACE_DELIMITERS);

        assert_eq!(result, Ok(Status::Complete(RESPONSE_WITH_MULTIPLE_SPACE_DELIMITERS.len())));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 0);
    }

    /// This is technically allowed by the spec, but we only support multiple spaces as an option,
    /// not stray `\r`s.
    static RESPONSE_WITH_WEIRD_WHITESPACE_DELIMITERS: &[u8] =
        b"HTTP/1.1 200\rOK\r\n\r\n";

    #[test]
    fn test_forbid_response_with_weird_whitespace_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_WEIRD_WHITESPACE_DELIMITERS);

        assert_eq!(result, Err(crate::Error::Status));
    }

    #[test]
    fn test_still_forbid_response_with_weird_whitespace_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_response_status_delimiters(true)
            .parse_response(&mut response, RESPONSE_WITH_WEIRD_WHITESPACE_DELIMITERS);
        assert_eq!(result, Err(crate::Error::Status));
    }

    static REQUEST_WITH_MULTIPLE_SPACE_DELIMITERS: &[u8] =
        b"GET  /    HTTP/1.1\r\n\r\n";

    #[test]
    fn test_forbid_request_with_multiple_space_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut request = Request::new(&mut headers[..]);
        let result = request.parse(REQUEST_WITH_MULTIPLE_SPACE_DELIMITERS);

        assert_eq!(result, Err(crate::Error::Token));
    }

    #[test]
    fn test_allow_request_with_multiple_space_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_request_line_delimiters(true)
            .parse_request(&mut request, REQUEST_WITH_MULTIPLE_SPACE_DELIMITERS);

        assert_eq!(result, Ok(Status::Complete(REQUEST_WITH_MULTIPLE_SPACE_DELIMITERS.len())));
        assert_eq!(request.method.unwrap(), "GET");
        assert_eq!(request.path.unwrap(), "/");
        assert_eq!(request.version.unwrap(), 1);
        assert_eq!(request.headers.len(), 0);
    }

    /// This is technically allowed by the spec, but we only support multiple spaces as an option,
    /// not stray `\r`s.
    static REQUEST_WITH_WEIRD_WHITESPACE_DELIMITERS: &[u8] =
        b"GET\r/\rHTTP/1.1\r\n\r\n";

    #[test]
    fn test_forbid_request_with_weird_whitespace_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut request = Request::new(&mut headers[..]);
        let result = request.parse(REQUEST_WITH_WEIRD_WHITESPACE_DELIMITERS);

        assert_eq!(result, Err(crate::Error::Token));
    }

    #[test]
    fn test_still_forbid_request_with_weird_whitespace_delimiters() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_request_line_delimiters(true)
            .parse_request(&mut request, REQUEST_WITH_WEIRD_WHITESPACE_DELIMITERS);
        assert_eq!(result, Err(crate::Error::Token));
    }

    static REQUEST_WITH_MULTIPLE_SPACES_AND_BAD_PATH: &[u8] = b"GET   /foo ohno HTTP/1.1\r\n\r\n";

    #[test]
    fn test_request_with_multiple_spaces_and_bad_path() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_request_line_delimiters(true)
            .parse_request(&mut request, REQUEST_WITH_MULTIPLE_SPACES_AND_BAD_PATH);
        assert_eq!(result, Err(crate::Error::Version));
    }

    // This test ensure there is an error when there is a DEL character in the path
    // since we allow all char from 0x21 code except DEL, this test ensure that DEL
    // is not allowed in the path
    static REQUEST_WITH_DEL_IN_PATH: &[u8] = b"GET   /foo\x7Fohno HTTP/1.1\r\n\r\n";

    #[test]
    fn test_request_with_del_in_path() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_request_line_delimiters(true)
            .parse_request(&mut request, crate::tests::REQUEST_WITH_DEL_IN_PATH);
        assert_eq!(result, Err(crate::Error::Token));
    }

    #[test]
    #[cfg_attr(miri, ignore)] // Miri is too slow for this test
    fn test_all_utf8_char_in_paths() {
        // two code points
        for i in 128..256 {
            for j in 128..256 {
                let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
                let mut request = Request::new(&mut headers[..]);
                let bytes = [i as u8, j as u8];

                match core::str::from_utf8(&bytes) {
                    Ok(s) => {
                        let first_line = format!("GET /{} HTTP/1.1\r\n\r\n", s);
                        let result = crate::ParserConfig::default()
                            .allow_multiple_spaces_in_request_line_delimiters(true)
                            .parse_request(&mut request, first_line.as_bytes());

                        assert_eq!(result, Ok(Status::Complete(20)), "failed for utf8 char i: {}, j: {}", i, j);
                    },
                    Err(_) => {
                        let mut first_line = b"GET /".to_vec();
                        first_line.extend(&bytes);
                        first_line.extend(b" HTTP/1.1\r\n\r\n");

                        let result = crate::ParserConfig::default()
                            .allow_multiple_spaces_in_request_line_delimiters(true)
                            .parse_request(&mut request, first_line.as_slice());

                        assert_eq!(result, Err(crate::Error::Token), "failed for utf8 char i: {}, j: {}", i, j);
                    },
                };

                // three code points starting from 0xe0
                if i < 0xe0 {
                    continue;
                }

                for k in 128..256 {
                    let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
                    let mut request = Request::new(&mut headers[..]);
                    let bytes = [i as u8, j as u8, k as u8];

                    match core::str::from_utf8(&bytes) {
                        Ok(s) => {
                            let first_line = format!("GET /{} HTTP/1.1\r\n\r\n", s);
                            let result = crate::ParserConfig::default()
                                .allow_multiple_spaces_in_request_line_delimiters(true)
                                .parse_request(&mut request, first_line.as_bytes());

                            assert_eq!(result, Ok(Status::Complete(21)), "failed for utf8 char i: {}, j: {}, k: {}", i, j, k);
                        },
                        Err(_) => {
                            let mut first_line = b"GET /".to_vec();
                            first_line.extend(&bytes);
                            first_line.extend(b" HTTP/1.1\r\n\r\n");

                            let result = crate::ParserConfig::default()
                                .allow_multiple_spaces_in_request_line_delimiters(true)
                                .parse_request(&mut request, first_line.as_slice());

                            assert_eq!(result, Err(crate::Error::Token), "failed for utf8 char i: {}, j: {}, k: {}", i, j, k);
                        },
                    };

                    // four code points starting from 0xf0
                    if i < 0xf0 {
                        continue;
                    }

                    for l in 128..256 {
                        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
                        let mut request = Request::new(&mut headers[..]);
                        let bytes = [i as u8, j as u8, k as u8, l as u8];

                        match core::str::from_utf8(&bytes) {
                            Ok(s) => {
                                let first_line = format!("GET /{} HTTP/1.1\r\n\r\n", s);
                                let result = crate::ParserConfig::default()
                                    .allow_multiple_spaces_in_request_line_delimiters(true)
                                    .parse_request(&mut request, first_line.as_bytes());

                                assert_eq!(result, Ok(Status::Complete(22)), "failed for utf8 char i: {}, j: {}, k: {}, l: {}", i, j, k, l);
                            },
                            Err(_) => {
                                let mut first_line = b"GET /".to_vec();
                                first_line.extend(&bytes);
                                first_line.extend(b" HTTP/1.1\r\n\r\n");

                                let result = crate::ParserConfig::default()
                                    .allow_multiple_spaces_in_request_line_delimiters(true)
                                    .parse_request(&mut request, first_line.as_slice());

                                assert_eq!(result, Err(crate::Error::Token), "failed for utf8 char i: {}, j: {}, k: {}, l: {}", i, j, k, l);
                            },
                        };
                    }
                }
            }
        }
    }

    static RESPONSE_WITH_SPACES_IN_CODE: &[u8] = b"HTTP/1.1 99 200 OK\r\n\r\n";

    #[test]
    fn test_response_with_spaces_in_code() {
        let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_multiple_spaces_in_response_status_delimiters(true)
            .parse_response(&mut response, RESPONSE_WITH_SPACES_IN_CODE);
        assert_eq!(result, Err(crate::Error::Status));
    }

    #[test]
    fn test_response_with_empty_header_name() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\n: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .allow_spaces_after_header_name_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Ok(Status::Complete(45)));

        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);
    }

    #[test]
    fn test_request_with_empty_header_name() {
        const RESPONSE: &[u8] =
            b"GET / HTTP/1.1\r\n: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, RESPONSE);
        assert_eq!(result, Ok(Status::Complete(44)));
    }

    #[test]
    fn test_request_with_whitespace_between_header_name_and_colon() {
        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials  : true\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .allow_spaces_after_header_name_in_responses(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()

            .ignore_invalid_headers_in_responses(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_response_with_invalid_char_between_header_name_and_colon() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials\xFF  : true\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .allow_spaces_after_header_name_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);

        assert_eq!(result, Ok(Status::Complete(79)));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);
    }

    #[test]
    fn test_request_with_invalid_char_between_header_name_and_colon() {
        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials\xFF  : true\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Ok(Status::Complete(78)));
    }

    #[test]
    fn test_ignore_header_line_with_missing_colon_in_response() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Ok(Status::Complete(70)));

        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);
    }

    #[test]
    fn test_ignore_header_line_with_missing_colon_in_request() {
        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Ok(Status::Complete(69)));
    }

    #[test]
    fn test_response_header_with_missing_colon_with_folding() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials   \r\n hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .allow_obsolete_multiline_headers_in_responses(true)
            .allow_spaces_after_header_name_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Ok(Status::Complete(81)));

        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);
    }

    #[test]
    fn test_request_header_with_missing_colon_with_folding() {
        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials   \r\n hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Ok(Status::Complete(80)));
    }

    #[test]
    fn test_response_header_with_nul_in_header_name() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Cred\0entials: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_request_header_with_nul_in_header_name() {
        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Cred\0entials: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_header_with_cr_in_header_name() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Cred\rentials: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Cred\rentials: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_header_with_nul_in_whitespace_before_colon() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials   \0: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .allow_spaces_after_header_name_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        let result = crate::ParserConfig::default()
            .allow_spaces_after_header_name_in_responses(true)
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderName));

        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials   \0: hello\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_header_with_nul_in_value() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials: hell\0o\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials: hell\0o\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderValue));
    }

    #[test]
    fn test_header_with_invalid_char_in_value() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials: hell\x01o\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Ok(Status::Complete(78)));

        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);

        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials: hell\x01o\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Ok(Status::Complete(77)));

        assert_eq!(request.version.unwrap(), 1);
        assert_eq!(request.method.unwrap(), "GET");
        assert_eq!(request.path.unwrap(), "/");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name, "Bread");
        assert_eq!(request.headers[0].value, &b"baguette"[..]);
    }

    #[test]
    fn test_header_with_invalid_char_in_value_with_folding() {
        const RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Credentials: hell\x01o  \n world!\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut response = Response::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_responses(true)
            .parse_response(&mut response, RESPONSE);
        assert_eq!(result, Ok(Status::Complete(88)));

        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Bread");
        assert_eq!(response.headers[0].value, &b"baguette"[..]);

        const REQUEST: &[u8] =
            b"GET / HTTP/1.1\r\nAccess-Control-Allow-Credentials: hell\x01o  \n world!\r\nBread: baguette\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 2];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Err(crate::Error::HeaderValue));

        let result = crate::ParserConfig::default()
            .ignore_invalid_headers_in_requests(true)
            .parse_request(&mut request, REQUEST);
        assert_eq!(result, Ok(Status::Complete(87)));

        assert_eq!(request.version.unwrap(), 1);
        assert_eq!(request.method.unwrap(), "GET");
        assert_eq!(request.path.unwrap(), "/");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name, "Bread");
        assert_eq!(request.headers[0].value, &b"baguette"[..]);
    }

    #[test]
    fn test_method_within_buffer() {
        const REQUEST: &[u8] = b"GET / HTTP/1.1\r\n\r\n";

        let mut headers = [EMPTY_HEADER; 0];
        let mut request = Request::new(&mut headers[..]);

        crate::ParserConfig::default()
            .parse_request(&mut request, REQUEST)
            .unwrap();

        // SAFETY: will not wrap
        let buf_end = unsafe { REQUEST.as_ptr().add(REQUEST.len()) };
        // Check that the method str is within the buffer
        let method = request.method.unwrap();
        assert!(REQUEST.as_ptr() <= method.as_ptr());
        assert!(method.as_ptr() <= buf_end);
    }

     static RESPONSE_WITH_SPACE_BEFORE_FIRST_HEADER: &[u8] =
        b"HTTP/1.1 200 OK\r\n Space-Before-Header: hello there\r\n\r\n";

    #[test]
    fn test_forbid_response_with_space_before_first_header() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = response.parse(RESPONSE_WITH_SPACE_BEFORE_FIRST_HEADER);

        assert_eq!(result, Err(crate::Error::HeaderName));
    }

    #[test]
    fn test_allow_response_response_with_space_before_first_header() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .allow_space_before_first_header_name(true)
            .parse_response(&mut response, RESPONSE_WITH_SPACE_BEFORE_FIRST_HEADER);

        assert_eq!(
            result,
            Ok(Status::Complete(
                RESPONSE_WITH_SPACE_BEFORE_FIRST_HEADER.len()
            ))
        );
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "Space-Before-Header");
        assert_eq!(response.headers[0].value, &b"hello there"[..]);
    }

    #[test]
    fn test_no_space_after_colon() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut response = Response::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .parse_response(&mut response, b"HTTP/1.1 200 OK\r\nfoo:bar\r\n\r\n");

        assert_eq!(result, Ok(Status::Complete(28)));
        assert_eq!(response.version.unwrap(), 1);
        assert_eq!(response.code.unwrap(), 200);
        assert_eq!(response.reason.unwrap(), "OK");
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name, "foo");
        assert_eq!(response.headers[0].value, &b"bar"[..]);
    }

    #[test]
    fn test_request_with_leading_space() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .parse_request(&mut request, b" GET / HTTP/1.1\r\nfoo:bar\r\n\r\n");

        assert_eq!(result, Err(Error::Token));
    }

    #[test]
    fn test_request_with_invalid_method() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut request = Request::new(&mut headers[..]);
        let result = crate::ParserConfig::default()
            .parse_request(&mut request, b"P()ST / HTTP/1.1\r\nfoo:bar\r\n\r\n");

        assert_eq!(result, Err(Error::Token));
    }

    #[test]
    fn test_utf8_in_path_ok() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default().parse_request(&mut request, b"GET /test?post=I\xE2\x80\x99msorryIforkedyou HTTP/1.1\r\nHost: example.org\r\n\r\n");

        assert_eq!(result, Ok(Status::Complete(67)));
        assert_eq!(request.version.unwrap(), 1);
        assert_eq!(request.method.unwrap(), "GET");
        assert_eq!(request.path.unwrap(), "/test?post=I’msorryIforkedyou");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name, "Host");
        assert_eq!(request.headers[0].value, &b"example.org"[..]);
    }

    #[test]
    fn test_bad_utf8_in_path() {
        let mut headers = [EMPTY_HEADER; 1];
        let mut request = Request::new(&mut headers[..]);

        let result = crate::ParserConfig::default().parse_request(&mut request, b"GET /test?post=I\xE2msorryIforkedyou HTTP/1.1\r\nHost: example.org\r\n\r\n");

        assert_eq!(result, Err(crate::Error::Token));
    }
}
