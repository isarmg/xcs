use sqlite_access::{Connection, Row, sqlite::SqlitePoolOptions};
use std::time::Duration;

#[tokio::test]
async fn pool_timeout_rollback_savepoint_and_statement_eviction() -> Result<(), sqlite_access::Error>
{
    let options = sqlite_access::sqlite::SqliteConnectOptions::new()
        .in_memory(true)
        .statement_cache_capacity(2);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(20))
        .connect_with(options)
        .await?;
    sqlite_access::query("CREATE TABLE entries (value TEXT NOT NULL)")
        .execute(&pool)
        .await?;
    let mut transaction = pool.begin().await?;
    assert!(matches!(
        pool.acquire().await,
        Err(sqlite_access::Error::PoolTimedOut)
    ));
    sqlite_access::query("INSERT INTO entries VALUES (?)")
        .bind("撤销 Ω")
        .execute(&mut *transaction)
        .await?;
    drop(transaction);
    let count: i64 = sqlite_access::query_scalar("SELECT count(*) FROM entries")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    transaction = pool.begin().await?;
    sqlite_access::query("INSERT INTO entries VALUES (?)")
        .bind("保留 Ω")
        .execute(&mut *transaction)
        .await?;
    let mut savepoint = transaction.begin().await?;
    sqlite_access::query("INSERT INTO entries VALUES (?)")
        .bind("嵌套撤销")
        .execute(&mut *savepoint)
        .await?;
    savepoint.rollback().await?;
    transaction.commit().await?;
    let values: Vec<String> = sqlite_access::query_scalar("SELECT value FROM entries")
        .fetch_all(&pool)
        .await?;
    assert_eq!(values, ["保留 Ω"]);
    let mut connection = pool.acquire().await?;
    for query in ["SELECT ?", "SELECT ? + 1", "SELECT ? + 2", "SELECT ? + 3"] {
        let _: i64 = sqlite_access::query_scalar(query)
            .bind(41_i64)
            .fetch_one(&mut *connection)
            .await?;
        assert!(connection.cached_statements_size() <= 2);
    }
    assert_eq!(connection.cached_statements_size(), 2);
    connection.clear_cached_statements().await?;
    assert_eq!(connection.cached_statements_size(), 0);
    drop(connection);
    pool.close().await;
    assert!(matches!(
        pool.acquire().await,
        Err(sqlite_access::Error::PoolClosed)
    ));
    Ok(())
}

#[tokio::test]
async fn sqlite_row_diagnostics_keep_values_null_and_decode_errors()
-> Result<(), sqlite_access::Error> {
    let mut connection = sqlite_access::SqliteConnection::connect("sqlite::memory:").await?;
    let row = sqlite_access::query("SELECT '文字 Ω' AS text_value, NULL AS null_value, X'00ff' AS blob_value, 1.25 AS real_value")
        .fetch_one(&mut connection).await?;
    let formatted = format!("{row:?}");
    assert!(formatted.contains("文字 Ω"), "{formatted}");
    assert!(formatted.contains("NULL"), "{formatted}");
    assert!(formatted.contains("[0, 255]"), "{formatted}");
    assert!(formatted.contains("1.25"), "{formatted}");
    assert!(row.try_get::<i64, _>("text_value").is_err());
    connection.close().await?;
    Ok(())
}
