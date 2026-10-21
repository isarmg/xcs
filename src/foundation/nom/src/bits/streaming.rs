//! Bit level parsers
//!

use crate::error::{ErrorKind, ParseError};
use crate::internal::{Err, IResult, Needed};
use crate::lib::std::ops::{AddAssign, Div, RangeFrom, Shl, Shr};
use crate::traits::{InputIter, InputLength, Slice, ToUsize};

/// Generates a parser taking `count` bits
pub fn take<I, O, C, E: ParseError<(I, usize)>>(
  count: C,
) -> impl Fn((I, usize)) -> IResult<(I, usize), O, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
  C: ToUsize,
  O: From<u8> + AddAssign + Shl<usize, Output = O> + Shr<usize, Output = O>,
{
  let count = count.to_usize();
  move |(input, bit_offset): (I, usize)| {
    if count == 0 {
      Ok(((input, bit_offset), 0u8.into()))
    } else {
      let cnt = (count + bit_offset).div(8);
      if input.input_len() * 8 < count + bit_offset {
        Err(Err::Incomplete(Needed::new(count as usize)))
      } else {
        let mut acc: O = 0_u8.into();
        let mut offset: usize = bit_offset;
        let mut remaining: usize = count;
        let mut end_offset: usize = 0;

        for byte in input.iter_elements().take(cnt + 1) {
          if remaining == 0 {
            break;
          }
          let val: O = if offset == 0 {
            byte.into()
          } else {
            ((byte << offset) as u8 >> offset).into()
          };

          if remaining < 8 - offset {
            acc += val >> (8 - offset - remaining);
            end_offset = remaining + offset;
            break;
          } else {
            acc += val << (remaining - (8 - offset));
            remaining -= 8 - offset;
            offset = 0;
          }
        }
        Ok(((input.slice(cnt..), end_offset), acc))
      }
    }
  }
}

/// Generates a parser taking `count` bits and comparing them to `pattern`
pub fn tag<I, O, C, E: ParseError<(I, usize)>>(
  pattern: O,
  count: C,
) -> impl Fn((I, usize)) -> IResult<(I, usize), O, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength + Clone,
  C: ToUsize,
  O: From<u8> + AddAssign + Shl<usize, Output = O> + Shr<usize, Output = O> + PartialEq,
{
  let count = count.to_usize();
  move |input: (I, usize)| {
    let inp = input.clone();

    take(count)(input).and_then(|(i, o)| {
      if pattern == o {
        Ok((i, o))
      } else {
        Err(Err::Error(error_position!(inp, ErrorKind::TagBits)))
      }
    })
  }
}

/// Parses one specific bit as a bool.
///
/// # Example
/// ```rust
/// # use nom::bits::complete::bool;
/// # use nom::IResult;
/// # use nom::error::{Error, ErrorKind};
///
/// fn parse(input: (&[u8], usize)) -> IResult<(&[u8], usize), bool> {
///     bool(input)
/// }
///
/// assert_eq!(parse(([0b10000000].as_ref(), 0)), Ok((([0b10000000].as_ref(), 1), true)));
/// assert_eq!(parse(([0b10000000].as_ref(), 1)), Ok((([0b10000000].as_ref(), 2), false)));
/// ```
pub fn bool<I, E: ParseError<(I, usize)>>(input: (I, usize)) -> IResult<(I, usize), bool, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let (res, bit): (_, u32) = take(1usize)(input)?;
  Ok((res, bit != 0))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/nom/unit/src/bits/streaming/test_105.rs"));
