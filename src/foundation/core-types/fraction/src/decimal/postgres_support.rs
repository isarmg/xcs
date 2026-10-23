extern crate bytes;

use self::bytes::BytesMut;
use ::numeric::collection::traits::Bounded;
use postgres_types::{FromSql, IsNull, ToSql, Type};

use std::error::Error;
use std::fmt;

use super::GenericDecimal;
use convert::TryToConvertFrom;
use fraction::GenericFraction;
use generic::GenericInteger;

use fraction::postgres_support::{fraction_to_sql_buf, read_i16, PG_MAX_PRECISION};

impl<'a, T, P> FromSql<'a> for GenericDecimal<T, P>
where
    T: Clone + GenericInteger + From<u16>,
    P: Copy + GenericInteger + Into<usize> + Bounded + TryToConvertFrom<i16> + fmt::Display,
{
    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        if raw.len() < 8 {
            return Err("unexpected data package from the database".into());
        }

        let scale: i16 = read_i16(&raw[6..8])?;
        let precision = if let Some(precision) = P::try_to_convert_from(scale) {
            precision
        } else {
            return Err(format!(
                "{} {};\n{} {};\n {} {};\n{} {}",
                r#"The precision of the source is too big: "#,
                scale,
                r#" your decimal type supports up to "#,
                P::max_value(),
                r#"you may increase the precision type size up to "usize", which is "#,
                usize::MAX,
                r#"PostgreSQL supports precision up to "#,
                PG_MAX_PRECISION
            )
            .into());
        };

        Ok(GenericDecimal(
            <GenericFraction<T> as FromSql>::from_sql(ty, raw)?,
            precision,
        ))
    }

    accepts!(NUMERIC);
}

impl<T, P> ToSql for GenericDecimal<T, P>
where
    T: Clone + GenericInteger + From<u8> + fmt::Debug,
    P: Copy + GenericInteger + Into<usize> + fmt::Debug,
{
    fn to_sql(
        &self,
        ty: &Type,
        buf: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        match *self {
            GenericDecimal(ref fraction, precision) => {
                fraction_to_sql_buf(fraction, ty, buf, precision.into())
            }
        }
    }

    accepts!(NUMERIC);

    to_sql_checked!();
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/decimal/postgres_support/tests_76.rs"));
