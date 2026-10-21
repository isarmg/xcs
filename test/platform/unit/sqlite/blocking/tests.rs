use super::*;
use sqlite_access::{
    Connection, SqliteConnection, sqlite::SqliteConnectOptions, sqlite::SqliteJournalMode,
};
use std::{fs, path::Path, time::Duration};

fn exercise_direct_connection(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    block_on_sqlite_connection(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await?;
        sqlite_access::raw_sql(
            "CREATE TABLE items(value INTEGER NOT NULL);INSERT INTO items VALUES(7)",
        )
        .execute(&mut connection)
        .await?;
        let value: i64 = sqlite_access::query_scalar("SELECT value FROM items")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(value, 7);
        // Closing the native worker, rather than dropping a pool, is the
        // required boundary before byte snapshots and permission changes.
        connection.close().await?;
        Ok::<_, sqlite_access::Error>(())
    })?;
    let bytes = fs::read(path)?;
    let wal = path.with_extension("sqlite3-wal");
    let shm = path.with_extension("sqlite3-shm");
    assert!(!wal.exists());
    assert!(!shm.exists());
    std::thread::sleep(Duration::from_millis(25));
    assert_eq!(fs::read(path)?, bytes);
    assert!(!wal.exists());
    assert!(!shm.exists());
    Ok(())
}

#[test]
fn direct_connection_without_an_async_runtime_closes_before_returning()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    exercise_direct_connection(&directory.path().join("outside.sqlite3"))
}

#[test]
fn direct_connection_inside_current_thread_runtime_closes_before_returning()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(async { exercise_direct_connection(&directory.path().join("current.sqlite3")) })
}

#[test]
fn direct_connection_inside_multi_thread_runtime_closes_before_returning()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()?
        .block_on(async { exercise_direct_connection(&directory.path().join("multi.sqlite3")) })
}

#[test]
fn index_connection_applies_fixed_pragmas_and_defensive_mode_without_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("index.sqlite3");
    block_on_sqlite_connection(async {
        let mut connection = crate::sqlite::open_index_connection(&path, true, None).await?;
        let mode: String = sqlite_access::query_scalar("PRAGMA journal_mode")
            .fetch_one(&mut connection)
            .await?;
        let synchronous: i64 = sqlite_access::query_scalar("PRAGMA synchronous")
            .fetch_one(&mut connection)
            .await?;
        let foreign_keys: i64 = sqlite_access::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(mode, "wal");
        assert_eq!(synchronous, 2);
        assert_eq!(foreign_keys, 1);
        sqlite_access::query("CREATE TABLE objects(value INTEGER)")
            .execute(&mut connection)
            .await?;
        sqlite_access::query("PRAGMA writable_schema=ON")
            .execute(&mut connection)
            .await?;
        assert!(
            sqlite_access::query("UPDATE sqlite_schema SET sql=NULL WHERE name='objects'")
                .execute(&mut connection)
                .await
                .is_err()
        );
        connection.close().await?;
        Ok::<_, crate::sqlite::Error>(())
    })?;
    Ok(())
}
