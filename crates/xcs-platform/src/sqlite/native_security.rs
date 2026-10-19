//! One-way connection hardening through SQLx's exclusive native handle.
use sqlx::{SqliteConnection, sqlite::LockedSqliteHandle};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionSecurityError {
    #[error("SQLite connection handle could not be acquired")]
    Connection(#[source] sqlx::Error),
    #[error("SQLite refused defensive mode")]
    DefensiveModeRejected,
}

/// Enable SQLite's defensive mode and read back the effective setting.
///
/// This is connection-local and cannot disable the setting. SQLx has no safe
/// db-config API, and SQLite has no equivalent defensive PRAGMA. Call before
/// product statements; a failed connection must not be used afterwards.
pub async fn enable_defensive(
    connection: &mut SqliteConnection,
) -> Result<(), ConnectionSecurityError> {
    let mut handle = connection
        .lock_handle()
        .await
        .map_err(ConnectionSecurityError::Connection)?;
    set_and_verify(&mut handle)
}

#[allow(unsafe_code)]
fn set_and_verify(handle: &mut LockedSqliteHandle<'_>) -> Result<(), ConnectionSecurityError> {
    let mut enabled: std::ffi::c_int = 0;
    // SAFETY: this live SQLx exclusive handle pauses its worker and uses the
    // same pinned native SQLite library. SQLITE_DBCONFIG_DEFENSIVE requires a
    // promoted C int and an initialized int output pointer. Only fixed 1 (set)
    // and -1 (read) are passed; output storage stays live and never escapes.
    let (set, read) = unsafe {
        let database = handle.as_raw_handle().as_ptr();
        let set = libsqlite3_sys::sqlite3_db_config(
            database,
            libsqlite3_sys::SQLITE_DBCONFIG_DEFENSIVE,
            1 as std::ffi::c_int,
            &mut enabled as *mut std::ffi::c_int,
        );
        let read = libsqlite3_sys::sqlite3_db_config(
            database,
            libsqlite3_sys::SQLITE_DBCONFIG_DEFENSIVE,
            -1 as std::ffi::c_int,
            &mut enabled as *mut std::ffi::c_int,
        );
        (set, read)
    };
    if set != libsqlite3_sys::SQLITE_OK || read != libsqlite3_sys::SQLITE_OK || enabled != 1 {
        return Err(ConnectionSecurityError::DefensiveModeRejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Connection;

    #[tokio::test]
    async fn defensive_mode_rejects_schema_writes_and_keeps_normal_sql_working()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut connection = SqliteConnection::connect("sqlite::memory:").await?;
        sqlx::query("CREATE TABLE items(value INTEGER)")
            .execute(&mut connection)
            .await?;
        let original = crate::sqlite::schema_fingerprint(&mut connection).await?;
        enable_defensive(&mut connection).await?;
        enable_defensive(&mut connection).await?;
        sqlx::query("PRAGMA writable_schema=ON")
            .execute(&mut connection)
            .await?;
        let result = sqlx::query("UPDATE sqlite_schema SET sql=NULL WHERE name='items'")
            .execute(&mut connection)
            .await;
        assert!(
            matches!(result, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("1"))
        );
        assert_eq!(
            crate::sqlite::schema_fingerprint(&mut connection).await?,
            original
        );
        sqlx::query("INSERT INTO items VALUES(7)")
            .execute(&mut connection)
            .await?;
        let value: i64 = sqlx::query_scalar("SELECT value FROM items")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(value, 7);
        connection.close().await?;

        // The setting is local: prove the same native schema write is possible
        // without defensive mode, rather than passing because of a bad query.
        let mut other = SqliteConnection::connect("sqlite::memory:").await?;
        sqlx::query("CREATE TABLE items(value INTEGER)")
            .execute(&mut other)
            .await?;
        sqlx::query("PRAGMA writable_schema=ON")
            .execute(&mut other)
            .await?;
        let changed = sqlx::query("UPDATE sqlite_schema SET sql=NULL WHERE name='items'")
            .execute(&mut other)
            .await?;
        assert_eq!(changed.rows_affected(), 1);
        other.close().await?;
        Ok(())
    }
}
