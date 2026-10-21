//! Asynchronous SQLite queries, pools, transactions and typed values.
#![recursion_limit = "512"]
#![deny(unsafe_code)]
#![warn(future_incompatible, rust_2018_idioms)]

#[macro_use]
mod query_engine;
pub use query_engine::*;
pub(crate) use query_engine::rt;

#[allow(unsafe_code)]
pub mod sqlite;


pub use query_engine::acquire::Acquire;
pub use query_engine::arguments::{Arguments, IntoArguments};
pub use query_engine::column::Column;
pub use query_engine::column::ColumnIndex;
pub use query_engine::column::ColumnOrigin;
pub use query_engine::connection::{ConnectOptions, Connection};
pub use query_engine::database::{self, Database};
pub use query_engine::describe::Describe;
pub use query_engine::executor::{Execute, Executor};
pub use query_engine::from_row::FromRow;
pub use query_engine::pool::{self, Pool};
#[doc(hidden)]
pub use query_engine::query::query_with_result as __query_with_result;
pub use query_engine::query::{query, query_with};
pub use query_engine::query_as::{query_as, query_as_with};
pub use query_engine::query_builder::{self, QueryBuilder};
#[doc(hidden)]
pub use query_engine::query_scalar::query_scalar_with_result as __query_scalar_with_result;
pub use query_engine::query_scalar::{query_scalar, query_scalar_with};
pub use query_engine::raw_sql::{raw_sql, RawSql};
pub use query_engine::row::Row;
pub use query_engine::sql_str::{AssertSqlSafe, SqlSafeStr, SqlStr};
pub use query_engine::statement::Statement;
pub use query_engine::transaction::Transaction;
pub use query_engine::type_info::TypeInfo;
pub use query_engine::types::Type;
pub use query_engine::value::{Value, ValueRef};
pub use query_engine::Either;

#[doc(inline)]
pub use query_engine::error::{self, Error, Result};

pub use sqlite::{
    Sqlite, SqliteConnection, SqliteExecutor, SqlitePool, SqliteTransaction,
};


pub use encode::Encode;
pub use decode::Decode;
/// Convenience re-export of common traits.
pub mod prelude {
    pub use super::Acquire;
    pub use super::ConnectOptions;
    pub use super::Connection;
    pub use super::Decode;
    pub use super::Encode;
    pub use super::Executor;
    pub use super::FromRow;
    pub use super::IntoArguments;
    pub use super::Row;
    pub use super::Statement;
    pub use super::Type;
}
