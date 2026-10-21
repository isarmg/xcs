use sqlx::{Connection, Row, sqlite::SqlitePoolOptions};
use std::time::Duration;

#[tokio::test]
async fn pool_timeout_rollback_savepoint_and_statement_eviction() -> Result<(), sqlx::Error> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .in_memory(true)
        .statement_cache_capacity(2);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(20))
        .connect_with(options)
        .await?;
    sqlx::query("CREATE TABLE entries (value TEXT NOT NULL)")
        .execute(&pool)
        .await?;
    let mut transaction = pool.begin().await?;
    assert!(matches!(
        pool.acquire().await,
        Err(sqlx::Error::PoolTimedOut)
    ));
    sqlx::query("INSERT INTO entries VALUES (?)")
        .bind("撤销 Ω")
        .execute(&mut *transaction)
        .await?;
    drop(transaction);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM entries")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    transaction = pool.begin().await?;
    sqlx::query("INSERT INTO entries VALUES (?)")
        .bind("保留 Ω")
        .execute(&mut *transaction)
        .await?;
    let mut savepoint = transaction.begin().await?;
    sqlx::query("INSERT INTO entries VALUES (?)")
        .bind("嵌套撤销")
        .execute(&mut *savepoint)
        .await?;
    savepoint.rollback().await?;
    transaction.commit().await?;
    let values: Vec<String> = sqlx::query_scalar("SELECT value FROM entries")
        .fetch_all(&pool)
        .await?;
    assert_eq!(values, ["保留 Ω"]);
    let mut connection = pool.acquire().await?;
    for query in ["SELECT ?", "SELECT ? + 1", "SELECT ? + 2", "SELECT ? + 3"] {
        let _: i64 = sqlx::query_scalar(query)
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
    assert!(matches!(pool.acquire().await, Err(sqlx::Error::PoolClosed)));
    Ok(())
}

#[tokio::test]
async fn sqlite_row_diagnostics_keep_values_null_and_decode_errors() -> Result<(), sqlx::Error> {
    let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:").await?;
    let row = sqlx::query("SELECT '文字 Ω' AS text_value, NULL AS null_value, X'00ff' AS blob_value, 1.25 AS real_value")
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
