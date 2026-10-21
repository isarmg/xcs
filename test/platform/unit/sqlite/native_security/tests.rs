use super::*;
use sqlite_access::Connection;

#[tokio::test]
async fn defensive_mode_rejects_schema_writes_and_keeps_normal_sql_working()
-> Result<(), Box<dyn std::error::Error>> {
    let mut connection = SqliteConnection::connect("sqlite::memory:").await?;
    sqlite_access::query("CREATE TABLE items(value INTEGER)")
        .execute(&mut connection)
        .await?;
    let original = crate::sqlite::schema_fingerprint(&mut connection).await?;
    enable_defensive(&mut connection).await?;
    enable_defensive(&mut connection).await?;
    sqlite_access::query("PRAGMA writable_schema=ON")
        .execute(&mut connection)
        .await?;
    let result = sqlite_access::query("UPDATE sqlite_schema SET sql=NULL WHERE name='items'")
        .execute(&mut connection)
        .await;
    assert!(
        matches!(result, Err(sqlite_access::Error::Database(ref error)) if error.code().as_deref() == Some("1"))
    );
    assert_eq!(
        crate::sqlite::schema_fingerprint(&mut connection).await?,
        original
    );
    sqlite_access::query("INSERT INTO items VALUES(7)")
        .execute(&mut connection)
        .await?;
    let value: i64 = sqlite_access::query_scalar("SELECT value FROM items")
        .fetch_one(&mut connection)
        .await?;
    assert_eq!(value, 7);
    connection.close().await?;

    // The setting is local: prove the same native schema write is possible
    // without defensive mode, rather than passing because of a bad query.
    let mut other = SqliteConnection::connect("sqlite::memory:").await?;
    sqlite_access::query("CREATE TABLE items(value INTEGER)")
        .execute(&mut other)
        .await?;
    sqlite_access::query("PRAGMA writable_schema=ON")
        .execute(&mut other)
        .await?;
    let changed = sqlite_access::query("UPDATE sqlite_schema SET sql=NULL WHERE name='items'")
        .execute(&mut other)
        .await?;
    assert_eq!(changed.rows_affected(), 1);
    other.close().await?;
    Ok(())
}
