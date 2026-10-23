use super::*;
use std::{io::Write, time::Instant};

fn source(size: usize) -> (tempfile::NamedTempFile, blake3::Hash) {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let block: Vec<u8> = (0..8192).map(|n| (n % 251) as u8).collect();
    let mut hash = blake3::Hasher::new();
    let mut left = size;
    while left > 0 {
        let bytes = &block[..left.min(block.len())];
        file.write_all(bytes).unwrap();
        hash.update(bytes);
        left -= bytes.len();
    }
    (file, hash.finalize())
}

// Cached local file + blocking-pool + stream overhead, NOT network throughput.
#[tokio::test]
#[ignore = "manual local streaming capacity comparison"]
async fn upload_capacity_measurement() {
    for size in [1024, 65536, 32 * 1024 * 1024] {
        let (file, expected) = source(size);
        let capacities = [4096, 16384, 65536, 262144];
        let mut timings = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        let mut counts = [0; 4];
        // Rotate execution order so competing work/cache warmth do not always
        // favor the larger capacities. Treat timings as diagnostic, not a gate.
        for round in 0..5 {
            for step in 0..4 {
                let index = (round + step) % 4;
                let capacity = capacities[index];
                let file = tokio::fs::File::open(file.path()).await.unwrap();
                let start = Instant::now();
                let mut stream = ReaderStream::with_capacity(file, capacity);
                let mut hash = blake3::Hasher::new();
                let mut chunks = 0;
                while let Some(chunk) = stream.next().await {
                    hash.update(&chunk.unwrap());
                    chunks += 1;
                }
                timings[index].push(start.elapsed().as_micros());
                counts[index] = chunks;
                assert_eq!(hash.finalize(), expected);
            }
        }
        for index in 0..4 {
            timings[index].sort_unstable();
            eprintln!(
                "size={size} capacity={} chunks={} median_us={}",
                capacities[index], counts[index], timings[index][2]
            );
        }
    }
}

#[test]
fn upload_capacity_is_nonzero_and_bounded_before_integer_conversion() {
    for (length, capacity) in [
        (0, 4096),
        (1, 4096),
        (4096, 4096),
        (4097, 4097),
        (65536, 65536),
        (262144, 262144),
        (262145, 262144),
        (u64::MAX, 262144),
    ] {
        assert_eq!(upload_chunk_capacity(length), capacity);
    }
}

#[tokio::test]
async fn upload_stream_preserves_bytes_and_bounds_chunks() {
    for size in [
        0, 1, 4095, 4096, 4097, 65536, 262143, 262144, 262145, 1048593,
    ] {
        let (file, expected) = source(size);
        let file = tokio::fs::File::open(file.path()).await.unwrap();
        let mut stream = upload_stream(file, size as u64);
        let mut hash = blake3::Hasher::new();
        let mut total = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            assert!(!chunk.is_empty());
            assert!(chunk.len() <= upload_chunk_capacity(size as u64));
            total += chunk.len();
            hash.update(&chunk);
        }
        assert_eq!(total, size);
        assert_eq!(hash.finalize(), expected);
        assert!(stream.next().await.is_none());
    }
}

#[tokio::test]
async fn upload_stream_propagates_real_file_errors_and_fuses() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let write_only = std::fs::OpenOptions::new()
        .write(true)
        .open(file.path())
        .unwrap();
    let mut stream = upload_stream(tokio::fs::File::from_std(write_only), 100);
    assert!(stream.next().await.unwrap().is_err());
    assert!(stream.next().await.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn upload_stream_does_not_read_without_demand_or_after_drop() {
    use std::io::Seek;
    let (file, _) = source(2 * 1024 * 1024);
    let file = std::fs::File::open(file.path()).unwrap();
    // The duplicated descriptor shares the offset, exposing actual reads.
    let mut observer = file.try_clone().unwrap();
    let mut stream = upload_stream(tokio::fs::File::from_std(file), 2 * 1024 * 1024);
    assert_eq!(observer.stream_position().unwrap(), 0);
    let first = stream.next().await.unwrap().unwrap();
    let position = observer.stream_position().unwrap();
    assert_eq!(position, first.len() as u64);
    tokio::task::yield_now().await;
    assert_eq!(observer.stream_position().unwrap(), position);
    drop(stream);
    tokio::task::yield_now().await;
    assert_eq!(observer.stream_position().unwrap(), position);
}

#[tokio::test]
async fn prepared_upload_retains_length_and_hash_headers() {
    let (file, hash) = source(262145);
    let client = Client::connect("http://127.0.0.1:1").unwrap();
    let file = Client::open_upload_source(file.path()).await.unwrap();
    let prepared = client
        .prepare_file_upload("ws", "item", "file", file)
        .await
        .unwrap();
    let request = prepared.request.build().unwrap();
    assert_eq!(request.headers()[reqwest::header::CONTENT_LENGTH], "262145");
    assert_eq!(request.headers()["x-zxs-logical-size"], "262145");
    assert_eq!(
        request.headers()["x-zxs-content-hash"],
        hash.to_hex().as_str()
    );
    assert!(request.body().unwrap().as_bytes().is_none());
}

#[tokio::test]
async fn upload_hash_rejects_short_or_growing_sources_with_bounded_reads() {
    for (content, expected_length) in [(b"short".as_slice(), 6), (b"growing".as_slice(), 3)] {
        let mut reader = content;
        assert!(matches!(
            hash_upload_source(&mut reader, expected_length).await,
            Err(Error::Response(message)) if message.contains("length changed")
        ));
        if expected_length == 3 {
            assert_eq!(
                reader, b"ing",
                "only one byte beyond the budget may be read"
            );
        }
    }
    // An infinite source must fail after the length plus a single probe, not
    // keep preparing forever. No file-system mutation race is needed here.
    let mut infinite = tokio::io::repeat(0);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        hash_upload_source(&mut infinite, 4),
    )
    .await
    .expect("upload preparation followed an unbounded source");
    assert!(matches!(result, Err(Error::Response(_))));
}

#[tokio::test]
async fn upload_hash_matches_exact_empty_and_nonempty_sources() {
    for content in [b"".as_slice(), b"immutable upload".as_slice()] {
        let mut reader = content;
        assert_eq!(
            hash_upload_source(&mut reader, content.len() as u64)
                .await
                .unwrap(),
            blake3::hash(content).to_hex().as_str()
        );
    }
}

#[tokio::test]
async fn upload_stream_never_emits_bytes_appended_beyond_prepared_length() {
    let (file, _) = source(8);
    let descriptor = Client::open_upload_source(file.path()).await.unwrap();
    let mut stream = upload_stream(descriptor, 8);
    std::fs::OpenOptions::new()
        .append(true)
        .open(file.path())
        .unwrap()
        .write_all(b"not part of the prepared upload")
        .unwrap();
    let mut received = Vec::new();
    while let Some(chunk) = stream.next().await {
        received.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(received, (0..8).collect::<Vec<u8>>());
}

#[tokio::test]
async fn upload_hash_preserves_source_io_errors() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let write_only = std::fs::OpenOptions::new()
        .write(true)
        .open(file.path())
        .unwrap();
    let mut reader = tokio::fs::File::from_std(write_only);
    assert!(matches!(
        hash_upload_source(&mut reader, 1).await,
        Err(Error::Io(_))
    ));
}
