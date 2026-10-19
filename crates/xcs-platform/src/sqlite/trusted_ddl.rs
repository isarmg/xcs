//! Bounded evaluation of product-owned trusted DDL for build fingerprints.

use crate::schema_identity::schema_fingerprint;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::time::{Duration, Instant};
use thiserror::Error;

const MAX_FRAGMENTS: usize = 128;
const MAX_DDL_BYTES: usize = 1024 * 1024;
use crate::sqlite::MAX_SCHEMA_BYTES;
const DEADLINE: Duration = Duration::from_secs(3);

#[derive(Debug, Error)]
pub enum DdlFingerprintError {
    #[error("trusted DDL input exceeds the supported bounds")]
    InputLimit,
    #[error("trusted DDL schema result exceeds the supported bounds")]
    ResultLimit,
    #[error("trusted DDL evaluation exceeded its deadline")]
    Deadline,
    #[error("trusted DDL SQLite evaluation failed")]
    Sqlite(#[source] sqlx::Error),
    #[error("trusted DDL schema fingerprint failed")]
    Schema(#[source] crate::schema_identity::Error),
    #[error("trusted DDL SQLite connection could not be closed")]
    Close(#[source] sqlx::Error),
}

/// Compute a schema fingerprint from owned, trusted product DDL synchronously.
///
/// This is a build-time adapter, not an arbitrary-SQL sandbox. Inputs must be
/// trusted source constants: SQL can access external paths (for example through
/// ATTACH), even though the primary connection is in memory. Never pass user,
/// configuration, downloaded or persisted SQL to this function.
///
/// At most 128 fragments / 1 MiB are accepted. Native execution has a three-second
/// progress deadline; results are limited to 1024 schema objects / 4 MiB of UTF-8
/// fields. The direct connection is explicitly closed on success and failure.
/// Metadata and SQLite internal objects are excluded by fingerprint version 1.
pub fn fingerprint_trusted_ddl(ddl: &[&str]) -> Result<String, DdlFingerprintError> {
    if ddl.len() > MAX_FRAGMENTS
        || ddl
            .iter()
            .try_fold(0usize, |total, fragment| total.checked_add(fragment.len()))
            .is_none_or(|total| total > MAX_DDL_BYTES)
    {
        return Err(DdlFingerprintError::InputLimit);
    }
    crate::sqlite::block_on_sqlite_connection(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .in_memory(true)
                .foreign_keys(true),
        )
        .await
        .map_err(DdlFingerprintError::Sqlite)?;
        let deadline = Instant::now() + DEADLINE;
        let result = evaluate(&mut connection, ddl, deadline).await;
        let close = connection.close().await;
        match (result, close) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(DdlFingerprintError::Close(error)),
            (Ok(fingerprint), Ok(())) => Ok(fingerprint),
        }
    })
}

async fn evaluate(
    connection: &mut SqliteConnection,
    ddl: &[&str],
    deadline: Instant,
) -> Result<String, DdlFingerprintError> {
    crate::sqlite::apply_connection_limits(
        connection,
        crate::sqlite::ConnectionLimits::new(MAX_SCHEMA_BYTES as u32),
    )
    .await
    .map_err(|_| DdlFingerprintError::InputLimit)?;
    {
        let mut handle = connection
            .lock_handle()
            .await
            .map_err(DdlFingerprintError::Sqlite)?;
        handle.set_progress_handler(1000, move || Instant::now() < deadline);
    }
    for fragment in ddl {
        if Instant::now() >= deadline {
            return Err(DdlFingerprintError::Deadline);
        }
        // Audited boundary: fragments are owned source constants, never data
        // received from configuration, persisted state, or external callers.
        sqlx::raw_sql(sqlx::AssertSqlSafe(*fragment))
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                if Instant::now() >= deadline {
                    DdlFingerprintError::Deadline
                } else {
                    DdlFingerprintError::Sqlite(error)
                }
            })?;
    }
    let rows = crate::sqlite::schema_rows(&mut *connection)
        .await
        .map_err(|error| {
            if Instant::now() >= deadline {
                return DdlFingerprintError::Deadline;
            }
            match error {
                crate::sqlite::Error::SchemaBudgetExceeded => DdlFingerprintError::ResultLimit,
                crate::sqlite::Error::Sqlx(error) => DdlFingerprintError::Sqlite(error),
                // schema_rows has no metadata or file-system operation.
                _ => DdlFingerprintError::ResultLimit,
            }
        })?;
    let fingerprint = schema_fingerprint(&rows).map_err(DdlFingerprintError::Schema)?;
    if Instant::now() >= deadline {
        return Err(DdlFingerprintError::Deadline);
    }
    Ok(fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_identity::SchemaRow;
    use crate::sqlite::MAX_SCHEMA_OBJECTS;

    #[test]
    fn fingerprints_canonical_schema_inside_and_outside_tokio()
    -> Result<(), Box<dyn std::error::Error>> {
        let ddl = [
            "CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            "CREATE INDEX items_value ON items(value)",
            crate::schema_identity::PRODUCT_METADATA_DDL,
        ];
        let expected = schema_fingerprint(&[
            SchemaRow::new(
                "index",
                "items_value",
                "items",
                "CREATE INDEX items_value ON items(value)",
            ),
            SchemaRow::new("table", "items", "items", ddl[0]),
        ])?;
        assert_eq!(fingerprint_trusted_ddl(&ddl)?, expected);
        tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(async {
                assert_eq!(fingerprint_trusted_ddl(&ddl).unwrap(), expected);
            });
        Ok(())
    }

    #[test]
    fn rejects_input_and_schema_object_excess_and_sql_errors() {
        assert!(matches!(
            fingerprint_trusted_ddl(&vec!["SELECT 1"; MAX_FRAGMENTS + 1]),
            Err(DdlFingerprintError::InputLimit)
        ));
        assert!(matches!(
            fingerprint_trusted_ddl(&[&"x".repeat(MAX_DDL_BYTES + 1)]),
            Err(DdlFingerprintError::InputLimit)
        ));
        assert!(matches!(
            fingerprint_trusted_ddl(&["CREATE not valid"]),
            Err(DdlFingerprintError::Sqlite(_))
        ));
        let ddl = (0..=MAX_SCHEMA_OBJECTS)
            .map(|index| format!("CREATE TABLE item_{index}(id INTEGER);"))
            .collect::<String>();
        assert!(matches!(
            fingerprint_trusted_ddl(&[&ddl]),
            Err(DdlFingerprintError::ResultLimit)
        ));
        // A failed evaluation is closed; a fresh evaluation remains usable.
        assert!(fingerprint_trusted_ddl(&["CREATE TABLE after_failure(id INTEGER)"]).is_ok());
    }

    #[test]
    fn native_progress_interrupts_unbounded_trusted_evaluation() {
        let start = Instant::now();
        assert!(matches!(
            fingerprint_trusted_ddl(&[
                "WITH RECURSIVE values_(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM values_) SELECT sum(n) FROM values_"
            ]),
            Err(DdlFingerprintError::Deadline)
        ));
        assert!(start.elapsed() < Duration::from_secs(10));
    }
}
