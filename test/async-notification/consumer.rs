use sqlite_access::sqlite::SqlitePoolOptions;
use std::time::Duration;

#[tokio::test]
async fn sqlite_close_wakes_acquirers_and_close_event_after_cancellation() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let held = pool.acquire().await.unwrap();
    let mut cancelled = Box::pin(pool.acquire());
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut cancelled)
            .await
            .is_err()
    );
    drop(cancelled);
    let first = pool.clone();
    let second = pool.clone();
    let first = tokio::spawn(async move { first.acquire().await });
    let second = tokio::spawn(async move { second.close_event().await });
    tokio::task::yield_now().await;
    let closing = pool.clone();
    let close = tokio::spawn(async move { closing.close().await });
    let result = tokio::time::timeout(Duration::from_secs(5), first)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err(sqlite_access::Error::PoolClosed)));
    tokio::time::timeout(Duration::from_secs(5), second)
        .await
        .unwrap()
        .unwrap();
    drop(held);
    tokio::time::timeout(Duration::from_secs(5), close)
        .await
        .unwrap()
        .unwrap();
    assert!(pool.is_closed());
}
