//! Strict SQLx adapters for ZXS's rebuildable metadata index.
//!
//! This crate does not own product DDL, migrations, locking, backup, restore or
//! file-permission policy. Opening an existing database and allowing SQLite to
//! create a missing file are deliberately separate operations.

use crate::schema_identity::{
    schema_fingerprint as fingerprint_rows, schema_identity_from_metadata_rows,
    validate_product_metadata_columns, validate_product_metadata_ddl,
};
use error_definition::Error;
use sqlite_access::{
    Connection, Executor, Row, Sqlite, SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{io, path::Path, path::PathBuf, time::Duration};

mod blocking;
pub use blocking::block_on_sqlite_connection;
mod native_limits;
pub use native_limits::{
    ConnectionLimitError, ConnectionLimits, EffectiveConnectionLimits, apply_connection_limits,
};
mod native_security;
pub use native_security::{ConnectionSecurityError, enable_defensive};
mod trusted_ddl;
pub use trusted_ddl::{DdlFingerprintError, fingerprint_trusted_ddl};

#[cfg(target_os = "linux")]
mod validation_snapshot;
#[cfg(target_os = "linux")]
pub use validation_snapshot::{
    SnapshotError, SnapshotLimits, ValidationSnapshot, ValidationSnapshotPool,
    open_validation_snapshot, open_validation_snapshot_with_connection_limits,
};

pub use crate::schema_identity::{
    Error as SchemaIdentityError, IdentityField, PRODUCT_METADATA_DDL, ProductMetadataColumn,
    ProductMetadataRow, SCHEMA_FINGERPRINT_ALGORITHM_VERSION, SchemaIdentity, SchemaRow,
};

pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum fingerprinted schema objects; an excess is rejected in full.
pub const MAX_SCHEMA_OBJECTS: usize = 1024;
/// Maximum aggregate UTF-8 bytes in the four fingerprinted fields.
pub const MAX_SCHEMA_BYTES: usize = 4 * 1024 * 1024;
/// Maximum retained foreign-key violation evidence, independent of row count.
pub const MAX_FOREIGN_KEY_EVIDENCE: usize = 32;

/// The intentionally small set of product-selectable pool settings.
///
/// Durability and correctness PRAGMAs are not configurable: every connection
/// uses WAL, foreign keys, a five-second busy timeout and FULL synchronous.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolOptions {
    max_connections: u32,
    min_connections: u32,
    acquire_timeout: Duration,
    connection_limits: Option<ConnectionLimits>,
}

impl PoolOptions {
    pub const fn new(max_connections: u32) -> Self {
        Self {
            max_connections,
            min_connections: 0,
            acquire_timeout: DEFAULT_ACQUIRE_TIMEOUT,
            connection_limits: None,
        }
    }

    pub const fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = min_connections;
        self
    }

    pub const fn with_acquire_timeout(mut self, acquire_timeout: Duration) -> Self {
        self.acquire_timeout = acquire_timeout;
        self
    }

    /// Apply native limits before product statements on every new connection,
    /// including connections replacing closed or failed pool members.
    pub const fn with_connection_limits(mut self, limits: ConnectionLimits) -> Self {
        self.connection_limits = Some(limits);
        self
    }

    pub const fn max_connections(&self) -> u32 {
        self.max_connections
    }

    pub const fn min_connections(&self) -> u32 {
        self.min_connections
    }

    pub const fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }

    pub fn validate(&self) -> Result<(), PoolOptionsError> {
        if self.max_connections == 0 {
            return Err(PoolOptionsError::ZeroMaxConnections);
        }
        if self.min_connections > self.max_connections {
            return Err(PoolOptionsError::MinConnectionsExceedMax {
                min_connections: self.min_connections,
                max_connections: self.max_connections,
            });
        }
        if self.acquire_timeout.is_zero() {
            return Err(PoolOptionsError::ZeroAcquireTimeout);
        }
        if let Some(limits) = self.connection_limits {
            limits
                .validate()
                .map_err(|_| PoolOptionsError::InvalidConnectionLimits)?;
        }
        Ok(())
    }
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self::new(10)
    }
}

/// Open an existing SQLite file. This operation never asks SQLite to create a
/// missing file and reports an initially missing path as a typed error.
pub async fn open_existing(
    database_path: impl AsRef<Path>,
    options: PoolOptions,
) -> Result<SqlitePool, Error> {
    options.validate()?;
    let database_path = database_path.as_ref();
    match database_path.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(Error::DatabaseDoesNotExist {
                path: database_path.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(Error::InspectDatabase {
                path: database_path.to_path_buf(),
                source,
            });
        }
    }
    connect(database_path, false, options).await
}

/// Open a SQLite file, explicitly allowing SQLite to create it when missing.
/// This is not a schema initializer and may also open an already-existing file.
pub async fn create_if_missing(
    database_path: impl AsRef<Path>,
    options: PoolOptions,
) -> Result<SqlitePool, Error> {
    options.validate()?;
    connect(database_path.as_ref(), true, options).await
}

async fn connect(
    database_path: &Path,
    create_if_missing: bool,
    options: PoolOptions,
) -> Result<SqlitePool, Error> {
    let connect_options = index_connect_options(database_path, create_if_missing);
    let pool = SqlitePoolOptions::new()
        .max_connections(options.max_connections)
        .min_connections(options.min_connections)
        .acquire_timeout(options.acquire_timeout)
        .after_connect(move |connection, _| {
            Box::pin(async move {
                // The query index never needs writable_schema or raw page writes.
                enable_defensive(connection)
                    .await
                    .map_err(|error| sqlite_access::Error::Io(io::Error::other(error)))?;
                if let Some(limits) = options.connection_limits {
                    apply_connection_limits(connection, limits)
                        .await
                        .map_err(|error| sqlite_access::Error::Io(io::Error::other(error)))?;
                }
                Ok(())
            })
        })
        .connect_with(connect_options)
        .await
        .map_err(pool_connect_error)?;
    Ok(pool)
}

/// Fixed connection policy for the rebuildable ZXS index.
///
/// Exposed for the synchronous engine's direct-connection worker. This does not
/// create directories, initialize schema, or attest cross-disk commit state.
pub fn index_connect_options(
    database_path: &Path,
    create_if_missing: bool,
) -> SqliteConnectOptions {
    index_open_options(database_path, create_if_missing)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
}

// SQLx drops rather than awaits closure if a connect-time PRAGMA fails. Keep
// schema-reading PRAGMAs out of connect_with so callers own and close the native
// worker before returning any corruption error (and moving database sidecars).
fn index_open_options(database_path: &Path, create_if_missing: bool) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(create_if_missing)
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT)
}

/// Open a direct SQLite connection, suitable for [`block_on_sqlite_connection`].
/// The caller must explicitly close the connection before returning from its
/// future. Filesystem permissions and index reconstruction remain engine duties.
pub async fn open_index_connection(
    database_path: &Path,
    create_if_missing: bool,
    limits: Option<ConnectionLimits>,
) -> Result<SqliteConnection, Error> {
    // Reject configuration errors before opening or creating an index file.
    if let Some(limits) = limits {
        limits.validate()?;
    }
    let mut connection =
        SqliteConnection::connect_with(&index_open_options(database_path, create_if_missing))
            .await?;
    let initialized = async {
        sqlite_access::query("PRAGMA journal_mode=WAL")
            .execute(&mut connection)
            .await?;
        sqlite_access::query("PRAGMA synchronous=FULL")
            .execute(&mut connection)
            .await?;
        enable_defensive(&mut connection)
            .await
            .map_err(|error| sqlite_access::Error::Io(io::Error::other(error)))?;
        if let Some(limits) = limits {
            apply_connection_limits(&mut connection, limits)
                .await
                .map_err(|error| sqlite_access::Error::Io(io::Error::other(error)))?;
        }
        Ok::<_, sqlite_access::Error>(())
    }
    .await;
    if let Err(error) = initialized {
        if let Err(close_error) = connection.close().await {
            // A caller must not interpret the original corruption code as
            // permission to move files while native closure remains uncertain.
            return Err(pool_connect_error(sqlite_access::Error::Io(
                io::Error::other(close_error),
            )));
        }
        return Err(pool_connect_error(error));
    }
    Ok(connection)
}

/// Best-effort candidate reads must not hold the engine mutex for the normal
/// writer busy budget. Preserve defensive/WAL/FULL policy, never create a file.
pub async fn open_index_read_connection(database_path: &Path) -> Result<SqliteConnection, Error> {
    let options = index_open_options(database_path, false)
        .read_only(true)
        .busy_timeout(Duration::from_millis(20));
    let mut connection = SqliteConnection::connect_with(&options).await?;
    let initialized = async {
        sqlite_access::query("PRAGMA journal_mode=WAL")
            .execute(&mut connection)
            .await?;
        sqlite_access::query("PRAGMA synchronous=FULL")
            .execute(&mut connection)
            .await?;
        enable_defensive(&mut connection)
            .await
            .map_err(|error| sqlite_access::Error::Io(io::Error::other(error)))?;
        apply_connection_limits(
            &mut connection,
            ConnectionLimits::new(64 * 1024)
                .with_max_sql_bytes(16 * 1024)
                .with_max_vm_operations(10_000)
                .with_max_bind_parameters(16),
        )
        .await
        .map_err(|error| sqlite_access::Error::Io(io::Error::other(error)))?;
        Ok::<_, sqlite_access::Error>(())
    }
    .await;
    if let Err(error) = initialized {
        if let Err(close_error) = connection.close().await {
            return Err(pool_connect_error(sqlite_access::Error::Io(
                io::Error::other(close_error),
            )));
        }
        return Err(pool_connect_error(error));
    }
    Ok(connection)
}

fn pool_connect_error(error: sqlite_access::Error) -> Error {
    if let sqlite_access::Error::Io(source) = error {
        let kind = source.kind();
        if let Some(inner) = source.into_inner() {
            return match inner.downcast::<ConnectionLimitError>() {
                Ok(error) => Error::ConnectionLimits(*error),
                Err(inner) => Error::Sqlx(sqlite_access::Error::Io(io::Error::new(kind, inner))),
            };
        }
        return Error::Sqlx(sqlite_access::Error::Io(io::Error::from(kind)));
    }
    Error::Sqlx(error)
}

/// Require the complete integrity-check result to be the single value `ok`.
pub async fn integrity_check(pool: &SqlitePool) -> Result<(), Error> {
    let messages: Vec<String> = sqlite_access::query_scalar("PRAGMA integrity_check")
        .fetch_all(pool)
        .await?;
    if messages.len() != 1 || !messages[0].eq_ignore_ascii_case("ok") {
        return Err(Error::IntegrityCheckFailed { messages });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignKeyViolation {
    pub table: String,
    pub row_id: Option<i64>,
    pub parent: String,
    pub foreign_key_index: i64,
}

/// Require `PRAGMA foreign_key_check` to return no violations.
///
/// Failure retains at most the first 32 violations as evidence. It never treats
/// a partial evidence collection as a successful integrity check.
pub async fn foreign_key_check(pool: &SqlitePool) -> Result<(), Error> {
    let violations = sqlite_access::query(
        "SELECT \"table\", rowid, parent, fkid FROM pragma_foreign_key_check LIMIT 32",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        Ok(ForeignKeyViolation {
            table: row.try_get(0)?,
            row_id: row.try_get(1)?,
            parent: row.try_get(2)?,
            foreign_key_index: row.try_get(3)?,
        })
    })
    .collect::<Result<Vec<_>, sqlite_access::Error>>()?;
    if !violations.is_empty() {
        return Err(Error::ForeignKeyViolations { violations });
    }
    Ok(())
}

/// Truncate-checkpoint the WAL and reject both busy and incomplete results.
pub async fn checkpoint(pool: &SqlitePool) -> Result<(), Error> {
    let result: (i64, i64, i64) = sqlite_access::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
        .fetch_one(pool)
        .await?;
    validate_checkpoint_result(result)
}

fn validate_checkpoint_result(
    (busy, log_frames, checkpointed_frames): (i64, i64, i64),
) -> Result<(), Error> {
    if busy != 0 {
        return Err(Error::CheckpointBusy {
            log_frames,
            checkpointed_frames,
        });
    }
    if log_frames != checkpointed_frames {
        return Err(Error::CheckpointIncomplete {
            log_frames,
            checkpointed_frames,
        });
    }
    Ok(())
}

/// Read the canonically filtered and ordered rows using any SQLx SQLite
/// executor, including a pool, connection or transaction connection.
///
/// At most 1024 objects / 4 MiB of UTF-8 fields are accepted. The SQL projection
/// suppresses all text before transfer to the driver when either bound is
/// exceeded. No fingerprint is computed from a truncated subset. This bounds
/// the returned data, not SQLite's native schema loading or total process heap.
pub async fn schema_rows<'executor, E>(executor: E) -> Result<Vec<SchemaRow>, Error>
where
    E: Executor<'executor, Database = Sqlite>,
{
    // Keep the version-1 filter, raw SQL bytes and BINARY ordering unchanged.
    // LIMIT 1025 proves an object excess without transferring an unbounded
    // result; window totals gate every text field before SQLx allocates it.
    let rows = sqlite_access::query(
        "WITH bounded AS (\
           SELECT type, name, tbl_name, COALESCE(sql, '') AS sql \
           FROM sqlite_schema \
           WHERE name NOT GLOB 'sqlite_*' AND name <> 'product_metadata' \
           ORDER BY type, name, tbl_name LIMIT 1025\
         ), charged AS (\
           SELECT *, COUNT(*) OVER () AS object_count, \
             SUM(length(CAST(type AS BLOB)) + length(CAST(name AS BLOB)) + \
                 length(CAST(tbl_name AS BLOB)) + length(CAST(sql AS BLOB))) \
               OVER () AS total_bytes FROM bounded\
         ) SELECT \
           CASE WHEN object_count <= 1024 AND total_bytes <= 4194304 THEN type END, \
           CASE WHEN object_count <= 1024 AND total_bytes <= 4194304 THEN name END, \
           CASE WHEN object_count <= 1024 AND total_bytes <= 4194304 THEN tbl_name END, \
           CASE WHEN object_count <= 1024 AND total_bytes <= 4194304 THEN sql END, \
           object_count, total_bytes FROM charged ORDER BY type, name, tbl_name",
    )
    .fetch_all(executor)
    .await?;
    if let Some(row) = rows.first()
        && (row.try_get::<i64, _>(4)? > MAX_SCHEMA_OBJECTS as i64
            || row.try_get::<i64, _>(5)? > MAX_SCHEMA_BYTES as i64)
    {
        return Err(Error::SchemaBudgetExceeded);
    }
    let rows = rows
        .into_iter()
        .map(|row| {
            Ok(SchemaRow::new(
                row.try_get::<String, _>(0)?,
                row.try_get::<String, _>(1)?,
                row.try_get::<String, _>(2)?,
                row.try_get::<String, _>(3)?,
            ))
        })
        .collect::<Result<Vec<_>, sqlite_access::Error>>()?;
    Ok(rows)
}

/// Calculate the canonical schema fingerprint using an arbitrary SQLx SQLite
/// executor. The pure algorithm lives in `crate::schema_identity`.
pub async fn schema_fingerprint<'executor, E>(executor: E) -> Result<String, Error>
where
    E: Executor<'executor, Database = Sqlite>,
{
    Ok(fingerprint_rows(&schema_rows(executor).await?)?)
}

/// Validate the metadata table and return an identity only after its declared
/// fingerprint has been verified against the actual schema.
pub async fn read_schema_identity(
    connection: &mut SqliteConnection,
) -> Result<SchemaIdentity, Error> {
    validate_metadata_table(connection).await?;
    let metadata_rows = read_metadata_rows(connection).await?;
    let identity = schema_identity_from_metadata_rows(&metadata_rows)?;
    let actual_fingerprint = schema_fingerprint(&mut *connection).await?;
    identity.verify_fingerprint(&actual_fingerprint)?;
    Ok(identity)
}

/// Validate an identity against exact compiled current values.
pub async fn require_current_schema(
    connection: &mut SqliteConnection,
    expected: &SchemaIdentity,
) -> Result<SchemaIdentity, Error> {
    let actual = read_schema_identity(connection).await?;
    actual.require_exact(expected)?;
    Ok(actual)
}

/// Pool convenience wrapper around [`read_schema_identity`].
pub async fn read_pool_schema_identity(pool: &SqlitePool) -> Result<SchemaIdentity, Error> {
    let mut connection = pool.acquire().await?;
    read_schema_identity(&mut connection).await
}

/// Pool convenience wrapper around [`require_current_schema`].
pub async fn require_pool_current_schema(
    pool: &SqlitePool,
    expected: &SchemaIdentity,
) -> Result<SchemaIdentity, Error> {
    let mut connection = pool.acquire().await?;
    require_current_schema(&mut connection, expected).await
}

async fn validate_metadata_table(connection: &mut SqliteConnection) -> Result<(), Error> {
    let ddl: Option<String> = sqlite_access::query_scalar(
        "SELECT sql FROM sqlite_schema WHERE type='table' AND name='product_metadata'",
    )
    .fetch_optional(&mut *connection)
    .await?;
    let ddl = ddl.ok_or(Error::ProductMetadataTableMissing)?;
    validate_product_metadata_ddl(&ddl)?;

    let columns = sqlite_access::query(
        "SELECT cid, name, type, \"notnull\", dflt_value, pk \
         FROM pragma_table_info('product_metadata') ORDER BY cid",
    )
    .fetch_all(&mut *connection)
    .await?
    .into_iter()
    .map(|row| {
        Ok(ProductMetadataColumn {
            cid: row.try_get(0)?,
            name: row.try_get(1)?,
            declared_type: row.try_get(2)?,
            not_null: row.try_get(3)?,
            default_sql: row.try_get(4)?,
            primary_key_position: row.try_get(5)?,
        })
    })
    .collect::<Result<Vec<_>, sqlite_access::Error>>()?;
    validate_product_metadata_columns(&columns)?;
    Ok(())
}

async fn read_metadata_rows(
    connection: &mut SqliteConnection,
) -> Result<Vec<ProductMetadataRow>, Error> {
    let rows = sqlite_access::query(
        "SELECT typeof(singleton), singleton, typeof(application), application, \
                typeof(application_version), application_version, \
                typeof(schema_revision), schema_revision, \
                typeof(schema_sha256), schema_sha256 \
         FROM product_metadata ORDER BY singleton LIMIT 2",
    )
    .fetch_all(connection)
    .await?;

    let mut metadata = Vec::with_capacity(rows.len());
    for row in rows {
        for (field, index, expected) in [
            ("singleton", 0, "integer"),
            ("application", 2, "text"),
            ("application_version", 4, "text"),
            ("schema_revision", 6, "integer"),
            ("schema_sha256", 8, "text"),
        ] {
            let actual: String = row.try_get(index)?;
            if actual != expected {
                return Err(Error::ProductMetadataStorageClass {
                    field,
                    expected,
                    actual,
                });
            }
        }
        metadata.push(ProductMetadataRow {
            singleton: row.try_get(1)?,
            application: row.try_get(3)?,
            application_version: row.try_get(5)?,
            schema_revision: row.try_get(7)?,
            schema_sha256: row.try_get(9)?,
        });
    }
    Ok(metadata)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum PoolOptionsError {
    #[error("max_connections must be greater than zero")]
    ZeroMaxConnections,
    #[error(
        "min_connections ({min_connections}) must not exceed max_connections ({max_connections})"
    )]
    MinConnectionsExceedMax {
        min_connections: u32,
        max_connections: u32,
    },
    #[error("acquire_timeout must be greater than zero")]
    ZeroAcquireTimeout,
    #[error("connection limits are outside the supported bounds")]
    InvalidConnectionLimits,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    InvalidPoolOptions(#[from] PoolOptionsError),
    #[error("SQLite database does not exist: {path}", path = .path.display())]
    DatabaseDoesNotExist { path: PathBuf },
    #[error("cannot inspect SQLite database {path}: {source}", path = .path.display())]
    InspectDatabase { path: PathBuf, source: io::Error },
    #[error("SQLite operation failed: {0}")]
    Sqlx(#[from] sqlite_access::Error),
    #[error(transparent)]
    ConnectionLimits(#[from] ConnectionLimitError),
    #[error("SQLite integrity check failed: {messages:?}")]
    IntegrityCheckFailed { messages: Vec<String> },
    #[error("SQLite foreign-key check found violations: {violations:?}")]
    ForeignKeyViolations {
        violations: Vec<ForeignKeyViolation>,
    },
    #[error(
        "SQLite WAL checkpoint is busy ({checkpointed_frames}/{log_frames} frames checkpointed)"
    )]
    CheckpointBusy {
        log_frames: i64,
        checkpointed_frames: i64,
    },
    #[error(
        "SQLite WAL checkpoint was incomplete ({checkpointed_frames}/{log_frames} frames checkpointed)"
    )]
    CheckpointIncomplete {
        log_frames: i64,
        checkpointed_frames: i64,
    },
    #[error("database has no product_metadata table")]
    ProductMetadataTableMissing,
    #[error("SQLite schema exceeds the supported object or byte budget")]
    SchemaBudgetExceeded,
    #[error("product_metadata {field} must use SQLite storage class {expected}, found {actual}")]
    ProductMetadataStorageClass {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error(transparent)]
    SchemaIdentity(#[from] SchemaIdentityError),
}

#[cfg(test)]
#[path = "../../../../test/platform/unit/sqlite/tests.rs"]
mod tests;
