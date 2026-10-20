//! Connection-local SQLite limits; no process-global memory settings.

use sqlx::{SqliteConnection, sqlite::LockedSqliteHandle};
use thiserror::Error;

/// Limits to apply before preparing product statements on a connection.
///
/// `max_value_bytes` bounds SQLite string/BLOB values and encoded rows. These
/// settings do not bound total process memory, sorting work, or allocations
/// made while initially opening a database. Use query deadlines, bounded
/// results and private database/schema validation in addition to these limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionLimits {
    pub max_value_bytes: u32,
    pub max_sql_bytes: u32,
    pub max_vm_operations: u32,
    pub max_bind_parameters: u32,
}

impl ConnectionLimits {
    pub const fn new(max_value_bytes: u32) -> Self {
        Self {
            max_value_bytes,
            max_sql_bytes: 1024 * 1024,
            max_vm_operations: 1_000_000,
            max_bind_parameters: 999,
        }
    }

    pub const fn with_max_sql_bytes(mut self, value: u32) -> Self {
        self.max_sql_bytes = value;
        self
    }

    pub const fn with_max_vm_operations(mut self, value: u32) -> Self {
        self.max_vm_operations = value;
        self
    }

    pub const fn with_max_bind_parameters(mut self, value: u32) -> Self {
        self.max_bind_parameters = value;
        self
    }

    pub(crate) fn validate(self) -> Result<(), ConnectionLimitError> {
        if !(1024..=64 * 1024 * 1024).contains(&self.max_value_bytes)
            || !(1024..=64 * 1024 * 1024).contains(&self.max_sql_bytes)
            || !(1024..=10_000_000).contains(&self.max_vm_operations)
            || !(1..=32766).contains(&self.max_bind_parameters)
        {
            return Err(ConnectionLimitError::InvalidLimits);
        }
        Ok(())
    }
}

/// Values read back from the native connection after applying all four limits.
pub type EffectiveConnectionLimits = ConnectionLimits;

#[derive(Debug, Error)]
pub enum ConnectionLimitError {
    #[error("SQLite connection limits are outside the supported bounds")]
    InvalidLimits,
    #[error("SQLite did not accept the requested connection limits")]
    NotApplied,
    #[error("SQLite connection handle could not be acquired")]
    Connection(#[source] sqlx::Error),
}

/// Apply limits to one connection and verify the native values.
///
/// All inputs are validated before changing the connection. Apply this before
/// preparing statements: a VM-operation limit does not invalidate statements
/// compiled earlier. A native rejection leaves the connection with its bounded
/// effective values; callers must not use that connection after an error.
pub async fn apply_connection_limits(
    connection: &mut SqliteConnection,
    limits: ConnectionLimits,
) -> Result<EffectiveConnectionLimits, ConnectionLimitError> {
    limits.validate()?;
    let mut handle = connection
        .lock_handle()
        .await
        .map_err(ConnectionLimitError::Connection)?;
    let mut actual = [0; 4];
    for (index, (category, value)) in [
        (LimitCategory::Value, limits.max_value_bytes),
        (LimitCategory::Sql, limits.max_sql_bytes),
        (LimitCategory::Vm, limits.max_vm_operations),
        (LimitCategory::Parameters, limits.max_bind_parameters),
    ]
    .into_iter()
    .enumerate()
    {
        native_limit(&mut handle, category, value as i32);
        actual[index] = native_limit(&mut handle, category, -1);
        if actual[index] != value as i32 {
            return Err(ConnectionLimitError::NotApplied);
        }
    }
    Ok(ConnectionLimits {
        max_value_bytes: actual[0] as u32,
        max_sql_bytes: actual[1] as u32,
        max_vm_operations: actual[2] as u32,
        max_bind_parameters: actual[3] as u32,
    })
}

#[derive(Clone, Copy)]
enum LimitCategory {
    Value,
    Sql,
    Vm,
    Parameters,
}

#[allow(unsafe_code)]
fn native_limit(handle: &mut LockedSqliteHandle<'_>, category: LimitCategory, value: i32) -> i32 {
    let category = match category {
        LimitCategory::Value => libsqlite3_sys::SQLITE_LIMIT_LENGTH,
        LimitCategory::Sql => libsqlite3_sys::SQLITE_LIMIT_SQL_LENGTH,
        LimitCategory::Vm => libsqlite3_sys::SQLITE_LIMIT_VDBE_OP,
        LimitCategory::Parameters => libsqlite3_sys::SQLITE_LIMIT_VARIABLE_NUMBER,
    };
    // SAFETY: SQLx's live exclusive LockedSqliteHandle pauses its connection
    // worker for this borrow. SQLx and this crate use the same pinned native
    // SQLite library/type. The fixed category is valid, and value is either a
    // validated nonnegative i32 or -1 (read-only query). sqlite3_limit retains
    // neither this pointer nor any Rust storage; no handle escapes the borrow.
    unsafe { libsqlite3_sys::sqlite3_limit(handle.as_raw_handle().as_ptr(), category, value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Connection;

    #[test]
    fn huge_cells_are_rejected_without_affecting_another_connection()
    -> Result<(), Box<dyn std::error::Error>> {
        crate::sqlite::block_on_sqlite_connection(async {
            let mut first = SqliteConnection::connect("sqlite::memory:").await?;
            let mut second = SqliteConnection::connect("sqlite::memory:").await?;
            sqlx::raw_sql(
                "CREATE TABLE data(value BLOB);INSERT INTO data VALUES(zeroblob(2097152))",
            )
            .execute(&mut first)
            .await?;
            let requested = ConnectionLimits::new(64 * 1024);
            assert_eq!(
                apply_connection_limits(&mut first, requested).await?,
                requested
            );
            let error = sqlx::query_scalar::<_, Vec<u8>>("SELECT value FROM data")
                .fetch_one(&mut first)
                .await
                .unwrap_err();
            assert_eq!(
                error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .as_deref(),
                Some("18")
            );
            assert!(
                sqlx::query("INSERT INTO data VALUES(zeroblob(2097152))")
                    .execute(&mut first)
                    .await
                    .is_err()
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT 7")
                    .fetch_one(&mut first)
                    .await?,
                7
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT length(zeroblob(2097152))")
                    .fetch_one(&mut second)
                    .await?,
                2097152
            );
            first.close().await?;
            second.close().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn sql_and_parameter_limits_are_native_and_invalid_input_changes_nothing()
    -> Result<(), Box<dyn std::error::Error>> {
        crate::sqlite::block_on_sqlite_connection(async {
            let mut connection = SqliteConnection::connect("sqlite::memory:").await?;
            let limits = ConnectionLimits::new(1024 * 1024)
                .with_max_sql_bytes(1024)
                .with_max_bind_parameters(8);
            assert_eq!(
                apply_connection_limits(&mut connection, limits).await?,
                limits
            );
            assert!(matches!(
                apply_connection_limits(&mut connection, ConnectionLimits::new(1)).await,
                Err(ConnectionLimitError::InvalidLimits)
            ));
            {
                let mut handle = connection.lock_handle().await?;
                assert_eq!(native_limit(&mut handle, LimitCategory::Sql, -1), 1024);
                assert_eq!(native_limit(&mut handle, LimitCategory::Parameters, -1), 8);
            }
            // Test-owned fixed comment bytes, with no external SQL inputs.
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "SELECT 1 /*{}*/",
                    "x".repeat(1024)
                )))
                .execute(&mut connection)
                .await
                .is_err()
            );
            assert!(
                sqlx::query("SELECT ?9")
                    .execute(&mut connection)
                    .await
                    .is_err()
            );
            connection.close().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    }
}
