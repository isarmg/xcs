use crate::database::Database;
#[cfg(feature = "pool")]
use crate::pool::PoolConnection;
use std::ops::{Deref, DerefMut};

#[doc(hidden)]
pub enum TransactionConnection<'c, DB: Database> {
    #[allow(dead_code)]
    Connection(&'c mut DB::Connection),
    #[cfg(feature = "pool")]
    PoolConnection(PoolConnection<DB>),
}

impl<DB: Database> Deref for TransactionConnection<'_, DB> {
    type Target = DB::Connection;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            TransactionConnection::Connection(v) => v,
            #[cfg(feature = "pool")]
            TransactionConnection::PoolConnection(v) => v,
        }
    }
}

impl<DB: Database> DerefMut for TransactionConnection<'_, DB> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            TransactionConnection::Connection(v) => v,
            #[cfg(feature = "pool")]
            TransactionConnection::PoolConnection(v) => v,
        }
    }
}

#[cfg(feature = "pool")]
impl<DB: Database> From<PoolConnection<DB>> for TransactionConnection<'_, DB> {
    fn from(v: PoolConnection<DB>) -> Self {
        TransactionConnection::PoolConnection(v)
    }
}

impl<'c, DB: Database> From<&'c mut DB::Connection> for TransactionConnection<'c, DB> {
    fn from(v: &'c mut DB::Connection) -> Self {
        TransactionConnection::Connection(v)
    }
}
