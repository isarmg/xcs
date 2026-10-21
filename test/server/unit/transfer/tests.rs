use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CountingReader {
    inner: std::io::Cursor<Vec<u8>>,
    bytes: Arc<AtomicUsize>,
}
impl std::io::Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let n = std::io::Read::read(&mut self.inner, buffer)?;
        self.bytes.fetch_add(n, Ordering::SeqCst);
        Ok(n)
    }
}
impl Seek for CountingReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}
fn counted(bytes: Vec<u8>, count: Arc<AtomicUsize>) -> CountingReader {
    CountingReader {
        inner: std::io::Cursor::new(bytes),
        bytes: count,
    }
}
fn manifest(bytes: &[u8]) -> storage_contract::ChecksumManifest {
    storage_contract::ChecksumManifest {
        chunk_bytes: 65536,
        logical_size: bytes.len() as u64,
        chunks: bytes
            .chunks(65536)
            .map(|chunk| *blake3::hash(chunk).as_bytes())
            .collect(),
    }
}
#[test]
fn verified_ranges_read_only_touched_chunks_and_fallback_without_exposing_corruption() {
    let bytes = vec![42; 65536 * 4 + 7];
    let count = Arc::new(AtomicUsize::new(0));
    let mut reader = VerifiedRange::new(
        counted(bytes.clone(), count.clone()),
        vec![],
        manifest(&bytes),
        65535,
        2,
    )
    .unwrap();
    assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
    assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
    assert!(reader.next_chunk().unwrap().is_none());
    assert_eq!(count.load(Ordering::SeqCst), 2 * 65536);
    let mut corrupt = bytes.clone();
    corrupt[65536] = 0;
    let bad_count = Arc::new(AtomicUsize::new(0));
    let good_count = Arc::new(AtomicUsize::new(0));
    let mut reader = VerifiedRange::new(
        counted(corrupt.clone(), bad_count.clone()),
        vec![counted(bytes.clone(), good_count.clone())],
        manifest(&bytes),
        65537,
        1,
    )
    .unwrap();
    assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
    assert_eq!(bad_count.load(Ordering::SeqCst), 65536);
    assert_eq!(good_count.load(Ordering::SeqCst), 65536);
    let mut reader = VerifiedRange::new(
        counted(corrupt, Arc::default()),
        vec![],
        manifest(&bytes),
        65537,
        1,
    )
    .unwrap();
    assert_eq!(
        reader.next_chunk().unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    // A short tail checks only actual remaining bytes, not a padded chunk.
    let tail_count = Arc::new(AtomicUsize::new(0));
    let mut reader = VerifiedRange::new(
        counted(bytes.clone(), tail_count.clone()),
        vec![],
        manifest(&bytes),
        (bytes.len() - 1) as u64,
        1,
    )
    .unwrap();
    assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
    assert_eq!(tail_count.load(Ordering::SeqCst), 7);
}

struct ReadLease(Arc<AtomicUsize>);
impl Drop for ReadLease {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn content(lease: Arc<AtomicUsize>) -> Content {
    let mut file = tempfile::tempfile().unwrap();
    std::io::Write::write_all(&mut file, b"0123456789").unwrap();
    Content {
        checksum_manifest: None,
        fallback_files: Vec::new(),
        file,
        size: 10,
        hash: "hash".into(),
        lease: Box::new(ReadLease(lease)),
        download_name: None,
    }
}

#[tokio::test]
async fn verified_stream_preserves_lease_and_admission_until_consumption_or_drop() {
    for consume in [true, false] {
        let slots = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = slots.clone().try_acquire_owned().unwrap();
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut content = content(dropped.clone());
        content.checksum_manifest = Some(manifest(b"0123456789"));
        content.lease = Box::new((content.lease, permit));
        let response = content_response(content, Method::GET, HeaderMap::new()).unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        assert_eq!(slots.available_permits(), 0);
        if consume {
            assert_eq!(
                http_routing::body::to_bytes(response.into_body(), 8192)
                    .await
                    .unwrap(),
                "0123456789"
            );
        } else {
            drop(response);
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert_eq!(slots.available_permits(), 1);
    }
}

#[tokio::test]
async fn verification_failure_after_a_chunk_stays_a_stream_error() {
    let original = vec![42; 65536 * 2];
    let expected = manifest(&original);
    let mut corrupted = original.clone();
    corrupted[65536] = 0;
    let mut file = tempfile::tempfile().unwrap();
    std::io::Write::write_all(&mut file, &corrupted).unwrap();
    let dropped = Arc::new(AtomicUsize::new(0));
    let content = Content {
        checksum_manifest: Some(expected),
        fallback_files: Vec::new(),
        file,
        size: original.len() as u64,
        hash: blake3::hash(&original).to_hex().to_string(),
        lease: Box::new(ReadLease(dropped.clone())),
        download_name: None,
    };
    let response = content_response(content, Method::GET, HeaderMap::new()).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/octet-stream"
    );
    let mut stream = response.into_body().into_data_stream();
    assert_eq!(stream.next().await.unwrap().unwrap(), &original[..65536]);
    assert!(stream.next().await.unwrap().is_err());
    // No JSON envelope is appended to the partial binary representation.
    assert!(stream.next().await.is_none());
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn ranges_obey_boundaries_and_reject_non_decimal_syntax() {
    assert_eq!(parse_range("bytes=2-8", 5).unwrap(), (2, 4));
    assert_eq!(parse_range("bytes=-2", 5).unwrap(), (3, 4));
    assert_eq!(parse_range("bytes=3-", 5).unwrap(), (3, 4));
    assert_eq!(
        parse_range("bytes=-18446744073709551615", 5).unwrap(),
        (0, 4)
    );
    assert_eq!(
        parse_range("bytes=0-", u64::MAX).unwrap(),
        (0, u64::MAX - 1)
    );
    for invalid in [
        "bytes=5-",
        "bytes=0-1,3-4",
        "bytes=-0",
        "bytes=missing",
        "bytes=+1-2",
        "bytes=1-+2",
        "bytes=18446744073709551616-",
        "bytes= 1-2",
    ] {
        assert_eq!(
            parse_range(invalid, 5).unwrap_err().status,
            416,
            "{invalid}"
        );
    }
    assert_eq!(parse_range("bytes=0-0", 0).unwrap_err().status, 416);
}

#[tokio::test]
async fn malformed_duplicate_and_unsatisfiable_ranges_include_size_and_head_is_empty() {
    for method in [Method::GET, Method::HEAD] {
        for invalid in ["bytes=missing", "bytes=+1-2", "bytes=20-"] {
            let mut headers = HeaderMap::new();
            headers.insert(header::RANGE, HeaderValue::from_str(invalid).unwrap());
            let response =
                content_response(content(Arc::default()), method.clone(), headers).unwrap();
            assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
            assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */10");
            assert_eq!(response.headers()[header::ACCEPT_RANGES], "bytes");
            let body = http_routing::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap();
            assert_eq!(body.is_empty(), method == Method::HEAD);
        }
        let mut headers = HeaderMap::new();
        headers.append(header::RANGE, HeaderValue::from_static("bytes=0-1"));
        headers.append(header::RANGE, HeaderValue::from_static("bytes=2-3"));
        let response = content_response(content(Arc::default()), method.clone(), headers).unwrap();
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */10");
    }
}

#[tokio::test]
async fn download_lease_survives_until_body_consumption_or_cancellation() {
    for consume in [true, false] {
        let dropped = Arc::new(AtomicUsize::new(0));
        let response =
            content_response(content(dropped.clone()), Method::GET, HeaderMap::new()).unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        if consume {
            assert_eq!(
                http_routing::body::to_bytes(response.into_body(), 8192)
                    .await
                    .unwrap(),
                "0123456789"
            );
        } else {
            drop(response);
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn stale_if_range_returns_complete_content_and_head_retains_length() {
    let mut headers = HeaderMap::new();
    headers.insert(header::RANGE, HeaderValue::from_static("bytes=2-3"));
    headers.insert(header::IF_RANGE, HeaderValue::from_static("\"stale\""));
    let response = content_response(content(Arc::default()), Method::GET, headers).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        http_routing::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap(),
        "0123456789"
    );
    let dropped = Arc::new(AtomicUsize::new(0));
    let mut headers = HeaderMap::new();
    headers.insert(header::RANGE, HeaderValue::from_static("bytes=2-3"));
    let response = content_response(content(dropped.clone()), Method::HEAD, headers).unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_LENGTH], "2");
    assert!(
        http_routing::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}
