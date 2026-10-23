use crate::sqlite::arguments::SqliteArgumentsBuffer;
use crate::sqlite::{Sqlite, SqliteTypeInfo, SqliteValueRef};
use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::types::{Text, Type};
use std::fmt::Display;
use std::str::FromStr;

impl<T> Type<Sqlite> for Text<T> {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <String as Type<Sqlite>>::compatible(ty)
    }
}

impl<T> Encode<'_, Sqlite> for Text<T>
where
    T: Display,
{
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.0.to_string(), buf)
    }
}

impl<'r, T> Decode<'r, Sqlite> for Text<T>
where
    T: FromStr,
    BoxDynError: From<<T as FromStr>::Err>,
{
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(Self(value.with_temp_text(|text| text.parse::<T>())??))
    }
}
