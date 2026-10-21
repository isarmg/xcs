//! Value formatting used by SQLite row diagnostics.
use crate::{database::Database, decode::Decode, type_info::TypeInfo, value::Value};
use std::{
    any::Any,
    fmt::{self, Debug, Formatter},
};

pub trait ValueDebug: Database {
    fn fmt_value_debug(value: &Self::Value) -> FmtValue<'_, Self>;
}

/// An adapter for [`Value`] which attempts to decode the value and format it when printed using [`Debug`].
pub struct FmtValue<'v, DB>
where
    DB: Database,
{
    value: &'v <DB as Database>::Value,
    fmt: fn(&'v <DB as Database>::Value, &mut Formatter<'_>) -> fmt::Result,
}

impl<'v, DB> FmtValue<'v, DB>
where
    DB: Database,
{
    // This API can't take `ValueRef` directly as it would need to pass it to `Decode` by-value,
    // which means taking ownership of it. We cannot rely on a `Clone` impl because `SqliteValueRef` doesn't have one.
    /// When printed with [`Debug`], attempt to decode `value` as the given type `T` and format it using [`Debug`].
    ///
    /// If `value` could not be decoded as `T`, the reason is printed instead.
    pub fn debug<T>(value: &'v <DB as Database>::Value) -> Self
    where
        T: Decode<'v, DB> + Debug + Any,
    {
        Self {
            value,
            fmt: |value, f| {
                let info = value.type_info();

                match T::decode(value.as_ref()) {
                    Ok(value) => Debug::fmt(&value, f),
                    Err(e) => {
                        if e.is::<crate::error::UnexpectedNullError>() {
                            f.write_str("NULL")
                        } else {
                            f.write_fmt(format_args!(
                                "(error decoding SQL type {} as {}: {e:?})",
                                info.name(),
                                std::any::type_name::<T>()
                            ))
                        }
                    }
                }
            },
        }
    }

    /// If the type to be decoded is not known or not supported, print the SQL type instead,
    /// as well as any applicable SQLx feature that needs to be enabled.
    pub fn unknown(value: &'v <DB as Database>::Value) -> Self
    where
        DB: ValueDebug,
    {
        Self {
            value,
            fmt: |value, f| {
                let info = value.type_info();

                f.write_fmt(format_args!("(unknown SQL type {})", info.name()))
            },
        }
    }
}

impl<DB> Debug for FmtValue<'_, DB>
where
    DB: Database,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (self.fmt)(self.value, f)
    }
}
