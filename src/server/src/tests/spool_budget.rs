use super::*;
use crate::spool_budget::METADATA_ALLOWANCE;
use std::sync::atomic::{AtomicBool, Ordering};

fn config(root: &Path, bytes: u64) -> ServerConfig {
    let mut config = ServerConfig::loopback(root);
    config.http_spool_limits = SpoolLimits {
        bytes_per_account: bytes,
        bytes_total: bytes,
        recovery_reserve_bytes: 1,
    };
    config
}
fn request(
    token: &str,
    item: &str,
    generation: u64,
    length: u64,
    hash: &str,
    body: Body,
) -> Request {
    Request::builder()
        .method("PUT")
        .uri(format!("/api/workspaces/ws/files/{item}/content?path=file"))
        .header("authorization", format!("Bearer {token}"))
        .header("x-zxs-logical-size", length.to_string())
        .header("x-zxs-content-hash", hash)
        .header("x-zxs-upload-generation", generation.to_string())
        .body(body)
        .unwrap()
}
fn restart(root: &Path, bytes: u64) -> Router {
    router(
        Arc::new(TestBackend {
            path: root.join("content"),
        }),
        AuthStore::open(root).unwrap(),
        config(root, bytes),
    )
    .unwrap()
}
fn data_count(root: &Path) -> usize {
    std::fs::read_dir(root.join("api-spool"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "data")
        })
        .count()
}

#[tokio::test]
async fn completed_and_old_generations_use_quota_before_body_poll_and_survive_restart() {
    let root = tempfile::tempdir().unwrap();
    let limit = 2 * METADATA_ALLOWANCE + 8;
    let app = application_config(root.path(), config(root.path(), limit));
    let (token, _, _) = authenticated(&app).await;
    let hash = blake3::hash(b"data").to_hex().to_string();
    for generation in [1, 2, 2] {
        let response = app
            .clone()
            .oneshot(request(
                &token,
                "item",
                generation,
                4,
                &hash,
                Body::from("data"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(
        data_count(root.path()),
        2,
        "a retry reuses its physical file"
    );
    drop(app);
    let app = restart(root.path(), limit);
    // Login/session and control routes stay available even when the spool is full.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let polled = Arc::new(AtomicBool::new(false));
    let observed = polled.clone();
    let body = Body::from_stream(futures_util::stream::poll_fn(move |_| {
        observed.store(true, Ordering::SeqCst);
        std::task::Poll::Ready(Some(Ok::<_, std::io::Error>(Bytes::from_static(b"data"))))
    }));
    let response = app
        .clone()
        .oneshot(request(&token, "item", 3, 4, &hash, body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        !polled.load(Ordering::SeqCst),
        "quota admission must precede body consumption"
    );
    assert_eq!(data_count(root.path()), 2);
    let response = app
        .oneshot(request(&token, "item", 2, 4, &hash, Body::from("data")))
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "same-generation retry needs no second full-file budget"
    );
}

#[tokio::test]
async fn interrupted_and_checksum_failed_bytes_remain_charged_after_restart() {
    let root = tempfile::tempdir().unwrap();
    let limit = 2 * METADATA_ALLOWANCE + 8;
    let app = application_config(root.path(), config(root.path(), limit));
    let (token, _, _) = authenticated(&app).await;
    let body = Body::from_stream(futures_util::stream::iter([
        Ok(Bytes::from_static(b"abc")),
        Err(std::io::Error::other("disconnect")),
    ]));
    let response = app
        .clone()
        .oneshot(request(&token, "item", 1, 4, &"0".repeat(64), body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = app
        .clone()
        .oneshot(request(
            &token,
            "item",
            2,
            5,
            &"0".repeat(64),
            Body::from("wrong"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let record = uploads::load(&root.path().join("api-spool"), "ws", "item")
        .unwrap()
        .unwrap();
    assert_eq!(record.state, "checksum_mismatch");
    drop(app);
    let app = restart(root.path(), limit);
    let response = app
        .oneshot(request(
            &token,
            "another",
            1,
            0,
            blake3::hash(b"").to_hex().as_str(),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(data_count(root.path()), 2);
    assert_eq!(
        std::fs::read(root.path().join("api-spool").join(record.data_file)).unwrap(),
        b"wrong"
    );
}

#[tokio::test]
async fn cancellation_settles_actual_unknown_tail_and_retry_never_overwrites_it() {
    let root = tempfile::tempdir().unwrap();
    let limit = METADATA_ALLOWANCE + 10;
    let app = application_config(root.path(), config(root.path(), limit));
    let (token, _, _) = authenticated(&app).await;
    let expected = b"abcdefghij";
    let hash = blake3::hash(expected).to_hex().to_string();
    let stream = futures_util::StreamExt::chain(
        futures_util::stream::once(async { Ok::<_, std::io::Error>(Bytes::from_static(b"abc")) }),
        futures_util::stream::pending(),
    );
    let upload_app = app.clone();
    let upload_request = request(&token, "item", 1, 10, &hash, Body::from_stream(stream));
    let upload = tokio::spawn(async move { upload_app.oneshot(upload_request).await });
    let record = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(record) =
                uploads::load(&root.path().join("api-spool"), "ws", "item").unwrap()
                && std::fs::metadata(root.path().join("api-spool").join(&record.data_file))
                    .unwrap()
                    .len()
                    == 3
            {
                break record;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        record.persisted_bytes, 0,
        "small tail has not been declared durable"
    );
    let response = app
        .clone()
        .oneshot(request(
            &token,
            "other",
            1,
            0,
            blake3::hash(b"").to_hex().as_str(),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    upload.abort();
    let _ = upload.await;
    // A cancelled request can still have one bounded blocking job finishing.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = app
                .clone()
                .oneshot(request(
                    &token,
                    "item",
                    1,
                    10,
                    &hash,
                    Body::from("XYZdefghij"),
                ))
                .await
                .unwrap();
            if response.status() == StatusCode::CONFLICT {
                let bytes = axum::body::to_bytes(response.into_body(), 8192)
                    .await
                    .unwrap();
                let error: Value = serde_json::from_slice(&bytes).unwrap();
                if error["error"]["code"] == "UPLOAD_RANGE_CONFLICT" {
                    break;
                }
            } else {
                panic!("unexpected retry status: {}", response.status());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("api-spool").join(&record.data_file)).unwrap(),
        b"abc"
    );
    drop(app);
    let app = restart(root.path(), limit);
    let response = app
        .oneshot(request(
            &token,
            "item",
            1,
            10,
            &hash,
            Body::from(expected.as_slice()),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(data_count(root.path()), 1);
    assert_eq!(
        std::fs::read(root.path().join("api-spool").join(record.data_file)).unwrap(),
        expected
    );
}
