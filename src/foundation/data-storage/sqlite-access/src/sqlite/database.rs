pub(crate) use crate::database::{Database, HasStatementCache};

use crate::sqlite::arguments::SqliteArgumentsBuffer;
use crate::sqlite::{
    SqliteArguments, SqliteColumn, SqliteConnection, SqliteQueryResult, SqliteRow, SqliteStatement,
    SqliteTransactionManager, SqliteTypeInfo, SqliteValue, SqliteValueRef,
};

/// Sqlite database driver.
#[derive(Debug)]
pub struct Sqlite;

impl Database for Sqlite {
    type Connection = SqliteConnection;

    type TransactionManager = SqliteTransactionManager;

    type Row = SqliteRow;

    type QueryResult = SqliteQueryResult;

    type Column = SqliteColumn;

    type TypeInfo = SqliteTypeInfo;

    type Value = SqliteValue;
    type ValueRef<'r> = SqliteValueRef<'r>;

    type Arguments = SqliteArguments;
    type ArgumentBuffer = SqliteArgumentsBuffer;

    type Statement = SqliteStatement;

    const NAME: &'static str = "SQLite";

    const URL_SCHEMES: &'static [&'static str] = &["sqlite"];
}

impl HasStatementCache for Sqlite {}
