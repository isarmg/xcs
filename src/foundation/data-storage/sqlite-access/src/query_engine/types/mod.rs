//! Conversions between Rust and SQL types.
//!
//! For supported SQLite mappings, see [SQLite types](crate::sqlite::types).
//! The maintained package does not include other database drivers or SQLx derive/query macros.
//!
//! Any external types that have had [`Type`] implemented for, are re-exported in this module
//! for convenience as downstream users need to use a compatible version of the external crate
//! to take advantage of the implementation.
//!
//! # Nullable
//!
//! To represent nullable SQL types, `Option<T>` is supported where `T` implements `Type`.
//! An `Option<T>` represents a potentially `NULL` value from SQL.

use std::{borrow::Cow, rc::Rc, sync::Arc};

use crate::database::Database;
use crate::type_info::TypeInfo;

mod non_zero;

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
mod json;

mod text;

#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
#[doc(no_inline)]
pub use uuid::{self, Uuid};

#[cfg(feature = "bit-vec")]
#[cfg_attr(docsrs, doc(cfg(feature = "bit-vec")))]
#[doc(no_inline)]
pub use bit_vec::BitVec;

#[cfg(feature = "ipnet")]
#[cfg_attr(docsrs, doc(cfg(feature = "ipnet")))]
pub mod ipnet {
    #[doc(no_inline)]
    pub use ipnet::{IpNet, Ipv4Net, Ipv6Net};
}

#[cfg(feature = "json")]
pub use json::{Json, JsonRawValue, JsonValue};
pub use text::Text;

/// Indicates that a SQL type is supported for a database.
///
/// ## Custom SQLite types
///
/// Implement this trait manually to describe a custom type's SQLite representation.
/// Implement [`Encode`](crate::encode::Encode) to bind values and
/// [`Decode`](crate::decode::Decode) to read them. SQL and type compatibility are
/// checked at runtime; this package does not provide query or derive macros.
/// Delegate all three traits to an existing supported type for a transparent wrapper:
///
/// ```rust
/// use sqlite_access::{Database, Decode, Encode, Sqlite, Type};
/// use sqlite_access::encode::IsNull;
/// use sqlite_access::error::BoxDynError;
/// use sqlite_access::sqlite::{SqliteTypeInfo, SqliteValueRef};
///
/// #[derive(Debug, PartialEq)]
/// struct UserId(i64);
///
/// impl Type<Sqlite> for UserId {
///     fn type_info() -> SqliteTypeInfo {
///         <i64 as Type<Sqlite>>::type_info()
///     }
///
///     fn compatible(ty: &SqliteTypeInfo) -> bool {
///         <i64 as Type<Sqlite>>::compatible(ty)
///     }
/// }
///
/// impl<'q> Encode<'q, Sqlite> for UserId {
///     fn encode_by_ref(
///         &self,
///         buf: &mut <Sqlite as Database>::ArgumentBuffer,
///     ) -> Result<IsNull, BoxDynError> {
///         <i64 as Encode<'q, Sqlite>>::encode_by_ref(&self.0, buf)
///     }
/// }
///
/// impl<'r> Decode<'r, Sqlite> for UserId {
///     fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
///         <i64 as Decode<'r, Sqlite>>::decode(value).map(UserId)
///     }
/// }
///
/// # async fn round_trip(connection: &mut sqlite_access::SqliteConnection)
/// #     -> Result<(), sqlite_access::Error> {
/// let id: UserId = sqlite_access::query_scalar("SELECT ?")
///     .bind(UserId(42))
///     .fetch_one(connection)
///     .await?;
/// assert_eq!(id, UserId(42));
/// # Ok(())
/// # }
/// ```
///
/// Use [`query_scalar`](crate::query_scalar) for one column, or
/// [`query_as`](crate::query_as) with a tuple or a manual
/// [`FromRow`](crate::FromRow) implementation for multiple columns.
/// For enums, choose an integer or text representation and reject unknown values
/// in `Decode`; declaring `Type` alone does not perform that validation.
pub trait Type<DB: Database> {
    /// Returns the canonical SQL type for this Rust type.
    ///
    /// When binding arguments, this is used to tell the database what is about to be sent; which,
    /// the database then uses to guide query plans. This can be overridden by `Encode::produces`.
    ///
    /// Custom wrappers should use the type information of their encoded representation.
    fn type_info() -> DB::TypeInfo;

    /// Determines if this Rust type is compatible with the given SQL type.
    ///
    /// When decoding values from a row, this method is checked to determine if we should continue
    /// or raise a runtime type mismatch error.
    ///
    /// Defaults to checking [`TypeInfo::type_compatible()`].
    fn compatible(ty: &DB::TypeInfo) -> bool {
        Self::type_info().type_compatible(ty)
    }
}

// for references, the underlying SQL type is identical
impl<T: ?Sized + Type<DB>, DB: Database> Type<DB> for &'_ T {
    fn type_info() -> DB::TypeInfo {
        <T as Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <T as Type<DB>>::compatible(ty)
    }
}

// for optionals, the underlying SQL type is identical
impl<T: Type<DB>, DB: Database> Type<DB> for Option<T> {
    fn type_info() -> DB::TypeInfo {
        <T as Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        ty.is_null() || <T as Type<DB>>::compatible(ty)
    }
}

macro_rules! impl_type_for_smartpointer {
    ($smart_pointer:ty) => {
        impl<T, DB: Database> Type<DB> for $smart_pointer
        where
            T: Type<DB> + ?Sized,
        {
            fn type_info() -> DB::TypeInfo {
                <T as Type<DB>>::type_info()
            }

            fn compatible(ty: &DB::TypeInfo) -> bool {
                <T as Type<DB>>::compatible(ty)
            }
        }
    };
}

impl_type_for_smartpointer!(Arc<T>);
impl_type_for_smartpointer!(Box<T>);
impl_type_for_smartpointer!(Rc<T>);

impl<T, DB: Database> Type<DB> for Cow<'_, T>
where
    // `ToOwned` is required here to satisfy `Cow`
    T: Type<DB> + ToOwned + ?Sized,
{
    fn type_info() -> DB::TypeInfo {
        <T as Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <T as Type<DB>>::compatible(ty)
    }
}
