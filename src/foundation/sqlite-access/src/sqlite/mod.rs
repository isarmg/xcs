//! **SQLite** database driver.
//!
//! ### Note: `libsqlite3-sys` Version
//! This driver uses the `libsqlite3-sys` crate which links the native library for SQLite 3.
//! Only one version of `libsqlite3-sys` may appear in the dependency tree of your project.
//!
//! As of SQLx 0.9.0, the version of `libsqlite3-sys` is now a range instead of any specific version.
//! See the `libsqlite3-sys` dependency in `sqlite-access/Cargo.toml` for the current version range.
//!
//! If you are using `rusqlite` or any other crate that indirectly depends on `libsqlite3-sys`,
//! this should allow Cargo to select a compatible version.
//!
//! If Cargo **fails to select a compatible version**, this means the other crate is using
//! a `libsqlite3-sys` version outside of this range.
//!
//! We may increase the *maximum* version of the range at our discretion,
//! in patch (SemVer-compatible) releases, to allow users to upgrade to newer versions as desired.
//!
//! The *minimum* version of the range may be increased over time to drop very old or
//! insecure versions of SQLite, but this will only occur in major (SemVer-incompatible) releases.
//!
//! Note that this means a `cargo update` may increase the `libsqlite3-sys` version,
//! which could, in rare cases, break your build.
//!
//! To prevent this, you can pin the `libsqlite3-sys` version in your own dependencies:
//!
//! ```toml
//! [dependencies]
//! # for example, if 0.35.0 breaks the build
//! libsqlite3-sys = "0.34"
//! ```
//!
//! ### Static Linking (Default)
//! The `sqlite` feature enables the `bundled` feature of `libsqlite3-sys`,
//! which builds SQLite 3 from included source code and statically links it into the final binary.
//!
//! This requires some C build tools to be installed on the system; see
//! [the `rusqlite` README][rusqlite-readme-building] for details.
//!
//! This version of SQLite is generally much newer than system-installed versions of SQLite
//! (especially for LTS Linux distributions), and can be updated with a `cargo update`,
//! so this is the recommended option for ease of use and keeping up-to-date.
//!
//! ### Dynamic linking
//! To dynamically link to an existing SQLite library, the `sqlite-unbundled` feature can be used
//! instead.
//!
//! This allows updating SQLite independently of SQLx or using forked versions, but you must have
//! SQLite installed on the system or provide a path to the library at build time (see
//! [the `rusqlite` README][rusqlite-readme-building] for details).
//!
//! Note that this _may_ result in link errors if the SQLite version is too old,
//! or has [certain features disabled at compile-time](https://www.sqlite.org/compile.html).
//!
//! SQLite version `3.20.0` (released August 2018) or newer is recommended.
//!
//! **Please check your SQLite version and the flags it was built with before opening
//!   a GitHub issue because of errors in `libsqlite3-sys`.** Thank you.
//!
//! [rusqlite-readme-building]: https://github.com/rusqlite/rusqlite?tab=readme-ov-file#notes-on-building-rusqlite-and-libsqlite3-sys
//!
//! ### Optional Features
//!
//! The following features
//!

// SQLite is a C library. All interactions require FFI which is unsafe.
// All unsafe blocks should have comments pointing to SQLite docs and ensuring that we maintain
// invariants.
#![allow(unsafe_code)]


pub use arguments::{SqliteArgumentValue, SqliteArguments, SqliteArgumentsBuffer};
pub use column::SqliteColumn;
#[cfg(feature = "sqlite-deserialize")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite-deserialize")))]
pub use connection::deserialize::SqliteOwnedBuf;
#[cfg(feature = "sqlite-preupdate-hook")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite-preupdate-hook")))]
pub use connection::PreupdateHookResult;
pub use connection::{LockedSqliteHandle, SqliteConnection, SqliteOperation, UpdateHookResult};
pub use database::Sqlite;
pub use error::SqliteError;
pub use options::{
    SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqliteLockingMode, SqliteSynchronous,
};
pub use query_result::SqliteQueryResult;
pub use row::SqliteRow;
pub use statement::SqliteStatement;
pub use transaction::SqliteTransactionManager;
pub use type_info::SqliteTypeInfo;
pub use value::{SqliteValue, SqliteValueRef};

pub(crate) use crate::driver_prelude::*;

use crate::executor::Executor;

mod arguments;
mod column;
mod connection;
mod database;
mod error;
mod options;
mod query_result;
mod row;
mod statement;
mod transaction;
mod type_info;
pub mod types;
mod value;
mod value_debug;

#[cfg(feature = "regexp")]
mod regexp;

/// An alias for [`Pool`][crate::sqlite::pool::Pool], specialized for SQLite.
pub type SqlitePool = crate::sqlite::pool::Pool<Sqlite>;

/// An alias for [`PoolOptions`][crate::sqlite::pool::PoolOptions], specialized for SQLite.
pub type SqlitePoolOptions = crate::sqlite::pool::PoolOptions<Sqlite>;

/// An alias for [`Executor<'_, Database = Sqlite>`][Executor].
pub trait SqliteExecutor<'c>: Executor<'c, Database = Sqlite> {}
impl<'c, T: Executor<'c, Database = Sqlite>> SqliteExecutor<'c> for T {}

/// An alias for [`Transaction`][crate::transaction::Transaction], specialized for SQLite.
pub type SqliteTransaction<'c> = crate::transaction::Transaction<'c, Sqlite>;

// NOTE: required due to the lack of lazy normalization
impl_into_arguments_for_arguments!(SqliteArguments);
impl_column_index_for_row!(SqliteRow);
impl_column_index_for_statement!(SqliteStatement);
impl_acquire!(Sqlite, SqliteConnection);

// required because some databases have a different handling of NULL
impl_encode_for_option!(Sqlite);
