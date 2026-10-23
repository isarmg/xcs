extern crate byteorder;
extern crate bytes;

use self::byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use self::bytes::{BufMut, BytesMut};
use postgres_types::{FromSql, IsNull, ToSql, Type};

use crate::{GenericFraction, Sign, Zero};
use division::{divide_integral, divide_rem};
use generic::GenericInteger;

use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
const PG_POS: u16 = 0x0000;
const PG_NEG: u16 = 0x4000;
const PG_PINF: u16 = 0xD000;
const PG_NINF: u16 = 0xF000;
const PG_NAN: u16 = 0xC000;
const PG_NBASE_U: u16 = 10000;
const PG_NBASE_I: i16 = 10000;
pub const PG_MAX_PRECISION: usize = 16383;

#[inline]
pub fn read_i16(mut buf: &[u8]) -> Result<i16, Box<dyn Error + Sync + Send>> {
    match buf.read_i16::<BigEndian>() {
        Ok(n) => Ok(n),
        Err(e) => Err(e.into()),
    }
}

#[inline]
pub fn read_u16(mut buf: &[u8]) -> Result<u16, Box<dyn Error + Sync + Send>> {
    match buf.read_u16::<BigEndian>() {
        Ok(n) => Ok(n),
        Err(e) => Err(e.into()),
    }
}

#[inline]
pub fn write_i16(mut buf: &mut [u8], value: i16) -> Result<(), Box<dyn Error + Sync + Send>> {
    match buf.write_i16::<BigEndian>(value) {
        Ok(()) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[inline]
pub fn write_u16(mut buf: &mut [u8], value: u16) -> Result<(), Box<dyn Error + Sync + Send>> {
    match buf.write_u16::<BigEndian>(value) {
        Ok(()) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

impl<'a, T> FromSql<'a> for GenericFraction<T>
where
    T: Clone + GenericInteger + From<u16>,
{
    fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        if raw.len() < 8 {
            return Err("unexpected data package from the database".into());
        }

        let sign: u16 = read_u16(&raw[4..6])?;
        let sign = match sign {
            PG_NEG => Sign::Minus,
            PG_POS => Sign::Plus,
            PG_NAN => return Ok(Self::nan()),
            PG_PINF => return Ok(Self::infinity()),
            PG_NINF => return Ok(Self::neg_infinity()),
            _ => return Err("unexpected sign byte in the data package".into()),
        };

        let ndigits: i16 = read_i16(&raw[0..2])?;
        if ndigits <= 0 {
            return Ok(match sign {
                Sign::Minus => Self::neg_zero(),
                Sign::Plus => Self::zero(),
            });
        }

        let ndigits: usize = u16::try_from(ndigits)
            .map_err(|_| "data package declares negative digit count")?
            .into();
        if (raw.len() - 8) / 2 < ndigits {
            return Err("data package declares more digits than received".into());
        }

        let weight: i16 = read_i16(&raw[2..4])?;
        let uweight: usize = if weight <= 0 {
            0
        } else {
            u16::try_from(weight)
                .map_err(|_| "data package declares negative scale")?
                .into()
        };

        let mut num: T = 0u16.into();
        let mut den: T = 1u16.into();
        let mut exp: T = 1u16.into();

        let nbase: T = PG_NBASE_U.into();

        let overflow_message = "integer overflow during unpacking the database value (try to use bigger integer as the base for Fraction, or you may even try BigFraction to use heap memory)";

        if weight < 0 {
            for _ in weight..0 {
                den = match den.checked_mul(&nbase) {
                    Some(n) => n,
                    None => return Err(overflow_message.into()),
                };
            }
        } else {
            for _ in 0..weight {
                exp = match exp.checked_mul(&nbase) {
                    Some(n) => n,
                    None => return Err(overflow_message.into()),
                };
            }
        }

        for iteration in 0..ndigits {
            // TODO: check if we could do GCD it within the loop
            let i = 8 + iteration * 2;

            let digits: i16 = read_i16(&raw[i..i + 2])?;
            let mut digits = if digits < 0 {
                return Err("database sent unexpected negative value".into());
            } else {
                u16::try_from(digits).map_err(|_| "database sent unexpected negative value")?
            };

            /* Digit x000 */
            let digit: u16 = digits / 1000 * 1000;
            digits -= digit;

            let d: T = digit.into();
            let d = match d.checked_mul(&exp) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };

            num = match num.checked_add(&d) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };

            /* Digit 0x00 */
            let digit: u16 = digits / 100 * 100;
            digits -= digit;

            let d: T = digit.into();
            let d = match d.checked_mul(&exp) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };
            num = match num.checked_add(&d) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };

            /* Digit 00x0 */
            let digit: u16 = digits / 10 * 10;
            digits -= digit;

            let d: T = digit.into();
            let d = match d.checked_mul(&exp) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };
            num = match num.checked_add(&d) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };

            /* Digit 000x */
            let d: T = digits.into();
            let d = match d.checked_mul(&exp) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };
            num = match num.checked_add(&d) {
                Some(n) => n,
                None => return Err(overflow_message.into()),
            };

            /* maintain the exponential growth */
            if iteration >= uweight {
                // after the decimal point
                num = match num.checked_mul(&nbase) {
                    Some(n) => n,
                    None => return Err(overflow_message.into()),
                };

                den = match den.checked_mul(&nbase) {
                    Some(n) => n,
                    None => return Err(overflow_message.into()),
                };
            } else if uweight > 0 {
                // before the decimal point
                exp = match exp.checked_div(&nbase) {
                    Some(n) => n,
                    None => return Err(overflow_message.into()),
                };
            }
        }

        Ok(match sign {
            Sign::Plus => Self::new(num, den),
            Sign::Minus => Self::new_neg(num, den),
        })
    }

    accepts!(NUMERIC);
}

impl<T> ToSql for GenericFraction<T>
where
    T: Clone + GenericInteger + From<u8> + fmt::Debug,
{
    fn to_sql(
        &self,
        ty: &Type,
        buf: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        fraction_to_sql_buf(self, ty, buf, PG_MAX_PRECISION)
    }

    accepts!(NUMERIC);

    to_sql_checked!();
}

pub fn fraction_to_sql_buf<T>(
    source: &GenericFraction<T>,
    _ty: &Type,
    buf: &mut BytesMut,
    precision: usize,
) -> Result<IsNull, Box<dyn Error + Sync + Send>>
where
    T: Clone + GenericInteger + From<u8>,
{
    let precision = if precision <= PG_MAX_PRECISION {
        precision
    } else {
        PG_MAX_PRECISION
    };

    let buffer_offset: usize = buf.len();
    buf.put_u64(0); // fill in the first 8 bytes

    if source.is_zero() {
        return Ok(IsNull::No);
    }

    if source.is_nan() {
        write_u16(&mut buf[buffer_offset + 4..buffer_offset + 6], PG_NAN)?;
        return Ok(IsNull::No);
    }

    if source.is_infinite() {
        write_u16(
            &mut buf[buffer_offset + 4..buffer_offset + 6],
            match source.sign() {
                Some(Sign::Minus) => PG_NINF,
                _ => PG_PINF,
            },
        )?;
        return Ok(IsNull::No);
    }

    let numer: T = if let Some(n) = source.numer() {
        n.clone()
    } else {
        unreachable!();
    };
    let denom: T = if let Some(d) = source.denom() {
        d.clone()
    } else {
        unreachable!();
    };

    let mut ndigits: i16 = 0;
    let mut weight: i16 = -1;
    let mut scale: i16 = 0;
    let mut uscale: usize = 0;

    let mut ndigit: i16 = 0;
    let mut nptr: u32 = 0;

    let mut padding = true;
    let mut rpad = 0;

    let div_state = divide_integral(numer, denom, |digit: u8| {
        if padding && digit == 0 {
            return Ok(true);
        } else {
            padding = false;
        }

        let digit: i16 = digit.into();

        ndigit *= 10;
        ndigit += digit;

        nptr += 1;

        if nptr > 3 {
            nptr = 0;
            weight += 1;

            if ndigit == 0 {
                rpad += 1;
            } else {
                rpad = 0;
            }

            ndigits += 1;
            buf.put_i16(ndigit);
            ndigit = 0;
        }

        Ok(true)
    })?;

    if nptr != 0 {
        let shift = 4 - nptr;

        ndigits += 1;
        weight += 1;
        nptr = 0;

        let digits = (buf.len() - buffer_offset - 8) / 2;
        let mut rem_: i16 = 0;

        for i in 0..digits {
            let pos = buffer_offset + 8 + i * 2;
            let mut digit = read_i16(&buf[pos..pos + 2])?;

            let mut tmp_rem: i16 = (digit - (digit / 10 * 10)) * 1000;
            digit /= 10;

            if shift > 1 {
                tmp_rem /= 10;
                tmp_rem += (digit - (digit / 10 * 10)) * 1000;
                digit /= 10;

                if shift > 2 {
                    tmp_rem /= 10;
                    tmp_rem += (digit - (digit / 10 * 10)) * 1000;
                    digit /= 10;
                }
            }

            digit += rem_;
            rem_ = tmp_rem;

            write_i16(&mut buf[pos..pos + 2], digit)?;
        }

        ndigit += rem_;
        if ndigit == 0 {
            rpad += 1;
        } else {
            rpad = 0;
        }

        buf.put_i16(ndigit);
        ndigit = 0;
    }

    if div_state.remainder.is_zero() && rpad > 0 {
        ndigits -= rpad;
        buf.truncate(buffer_offset + 8 + (ndigits as usize) * 2);
    }

    if !div_state.remainder.is_zero() {
        padding = weight < 0; // true;
        divide_rem(
            div_state.remainder,
            div_state.divisor,
            |state, digit: u8| {
                let digit: i16 = digit.into();

                if digit != 0 {
                    if padding && weight > 0 {
                        ndigits += weight;
                        for _ in 0..weight {
                            buf.put_i16(ndigit);
                        }
                    }
                    padding = false;
                }

                nptr += 1;

                ndigit += digit * (PG_NBASE_I / 10i16.pow(nptr));

                scale += 1;
                uscale += 1;

                if nptr > 3 {
                    if padding && weight < 0 {
                        weight -= 1;
                    } else {
                        ndigits += 1;
                        buf.put_i16(ndigit);
                    }

                    nptr = 0;
                    ndigit = 0;
                }

                Ok(if uscale < precision {
                    Ok(state)
                } else {
                    Err(state)
                })
            },
        )?;

        if nptr != 0 && !padding {
            ndigits += 1;
            buf.put_i16(ndigit);
        }
    }

    write_i16(&mut buf[buffer_offset..buffer_offset + 2], ndigits)?;
    write_i16(&mut buf[buffer_offset + 2..buffer_offset + 4], weight)?;
    write_u16(
        &mut buf[buffer_offset + 4..buffer_offset + 6],
        match source.sign() {
            Some(Sign::Minus) => PG_NEG,
            _ => PG_POS,
        },
    )?;

    write_i16(&mut buf[buffer_offset + 6..buffer_offset + 8], scale)?;

    Ok(IsNull::No)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fraction/unit/src/fraction/postgres_support/tests_444.rs"));
