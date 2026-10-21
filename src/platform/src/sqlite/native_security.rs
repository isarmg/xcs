//! One-way connection hardening through SQLx's exclusive native handle.
use error_definition::Error;
use sqlx::{SqliteConnection, sqlite::LockedSqliteHandle};

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
#[path = "../../../../test/platform/unit/sqlite/native_security/tests.rs"]
mod tests;
