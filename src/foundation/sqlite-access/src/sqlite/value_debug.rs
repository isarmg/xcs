use crate::sqlite::Sqlite;
use crate::{
    types::Type,
    value::Value,
    value_debug::{FmtValue, ValueDebug},
};

impl ValueDebug for Sqlite {
    fn fmt_value_debug(value: &Self::Value) -> FmtValue<'_, Self> {
        let info = value.type_info();
        // Keep the original diagnostic selection order, including integer coercion.
        macro_rules! debug_types {
            ($($ty:ty),* $(,)?) => { $(
                if <$ty as Type<Sqlite>>::compatible(&info) {
                    return FmtValue::debug::<$ty>(value);
                }
            )* };
        }
        debug_types!(bool, i32, i64, f64, String, Vec<u8>);
        #[cfg(feature = "uuid")]
        debug_types!(crate::types::Uuid);
        FmtValue::unknown(value)
    }
}
