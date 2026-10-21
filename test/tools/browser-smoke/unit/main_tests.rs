fn decode_http(response: &[u8]) -> Result<(u16, Vec<u8>)> {
    read_http(&mut std::io::Cursor::new(response))
}

use super::*;
#[test]
fn chooser_interception_cleanup_runs_after_success_and_failure() {
    for succeed in [true, false] {
        let cleaned = std::cell::Cell::new(false);
        let result = with_cleanup(
            || {
                if succeed {
                    Ok(())
                } else {
                    Err("selection failed".into())
                }
            },
            || {
                cleaned.set(true);
                Ok(())
            },
        );
        assert!(cleaned.get());
        assert_eq!(result.is_ok(), succeed);
    }
    assert!(with_cleanup(|| Ok(()), || Err("cleanup failed".into())).is_err());
}
#[test]
fn webdriver_http_keeps_binary_body_and_status() {
    let (status, body) =
        decode_http(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n\x00\xff\x01").unwrap();
    assert_eq!(status, 200);
    assert_eq!(body, [0, 255, 1]);
    assert_eq!(
        decode_http(b"HTTP/1.1 500 Error\r\nContent-Length: 2\r\n\r\n{}")
            .unwrap()
            .0,
        500
    );
}
#[test]
fn malformed_or_unsupported_http_fails_closed() {
    assert!(decode_http(b"not HTTP").is_err());
    assert!(decode_http(b"HTTP/1.1 nope\r\n\r\n{}").is_err());
    assert!(
        decode_http(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n").is_err()
    );
}
#[test]
fn content_length_completes_without_waiting_for_keep_alive_eof() {
    // Like a live keep-alive socket: partial reads are possible; reading past
    // the complete response would block instead of returning EOF.
    struct KeepAlive(std::io::Cursor<Vec<u8>>);
    impl Read for KeepAlive {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            if self.0.position() == self.0.get_ref().len() as u64 {
                return Err(std::io::ErrorKind::WouldBlock.into());
            }
            let limit = output.len().min(3);
            self.0.read(&mut output[..limit])
        }
    }
    let response =
        b"HTTP/1.1 200 OK\r\ncontent-length: 14\r\nConnection: keep-alive\r\n\r\n{\"value\":true}";
    let mut socket = KeepAlive(std::io::Cursor::new(response.to_vec()));
    let (status, body) = read_http(&mut socket).unwrap();
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"value":true}"#);
}
#[test]
fn framing_rejects_truncated_missing_duplicate_or_excessive_lengths() {
    for response in [
        &b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n{}"[..],
        &b"HTTP/1.1 200 OK\r\n\r\n{}"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Length: 999999999\r\n\r\n"[..],
    ] {
        assert!(decode_http(response).is_err());
    }
    assert_eq!(
        decode_http(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap(),
        (204, vec![])
    );
    assert_eq!(
        decode_http(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}next response")
            .unwrap()
            .1,
        b"{}"
    );
}
