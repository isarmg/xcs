use crate::sqlite::arguments::SqliteArgumentsBuffer;
use crate::sqlite::decode::Decode;
use crate::sqlite::encode::{Encode, IsNull};
use crate::sqlite::error::BoxDynError;
use crate::sqlite::type_info::DataType;
use crate::sqlite::types::Type;
use crate::sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};

impl Type<Sqlite> for bool {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Bool)
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        matches!(ty.0, DataType::Bool | DataType::Int4 | DataType::Integer)
    }
}

impl Encode<'_, Sqlite> for bool {
    fn encode_by_ref(&self, args: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        args.push(SqliteArgumentValue::Int((*self).into()));

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r, Sqlite> for bool {
    fn decode(value: SqliteValueRef<'r>) -> Result<bool, BoxDynError> {
        Ok(value.int64()? != 0)
    }
}
