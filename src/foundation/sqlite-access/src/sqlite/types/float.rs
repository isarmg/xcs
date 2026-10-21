use crate::sqlite::arguments::SqliteArgumentsBuffer;
use crate::sqlite::decode::Decode;
use crate::sqlite::encode::{Encode, IsNull};
use crate::sqlite::error::BoxDynError;
use crate::sqlite::type_info::DataType;
use crate::sqlite::types::Type;
use crate::sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};

impl Type<Sqlite> for f32 {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Float)
    }
}

impl Encode<'_, Sqlite> for f32 {
    fn encode_by_ref(&self, args: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        args.push(SqliteArgumentValue::Double((*self).into()));

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r, Sqlite> for f32 {
    fn decode(value: SqliteValueRef<'r>) -> Result<f32, BoxDynError> {
        // Truncation is intentional
        #[allow(clippy::cast_possible_truncation)]
        Ok(value.double()? as f32)
    }
}

impl Type<Sqlite> for f64 {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Float)
    }
}

impl Encode<'_, Sqlite> for f64 {
    fn encode_by_ref(&self, args: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        args.push(SqliteArgumentValue::Double(*self));

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r, Sqlite> for f64 {
    fn decode(value: SqliteValueRef<'r>) -> Result<f64, BoxDynError> {
        Ok(value.double()?)
    }
}
