//! Parsers recognizing numbers, complete input version

use crate::branch::alt;
use crate::bytes::complete::tag;
use crate::character::complete::{char, digit1, sign};
use crate::combinator::{cut, map, opt, recognize};
use crate::error::ParseError;
use crate::error::{make_error, ErrorKind};
use crate::internal::*;
use crate::lib::std::ops::{Range, RangeFrom, RangeTo};
use crate::sequence::{pair, tuple};
use crate::traits::{
  AsBytes, AsChar, Compare, InputIter, InputLength, InputTake, InputTakeAtPosition, Offset, Slice,
};

/// Recognizes an unsigned 1 byte integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_u8;
///
/// let parser = |s| {
///   be_u8(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
/// assert_eq!(parser(&b""[..]), Err(Err::Error((&[][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_u8<I, E: ParseError<I>>(input: I) -> IResult<I, u8, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 1;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let res = input.iter_elements().next().unwrap();

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a big endian unsigned 2 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_u16;
///
/// let parser = |s| {
///   be_u16(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_u16<I, E: ParseError<I>>(input: I) -> IResult<I, u16, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 2;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u16;
    for byte in input.iter_elements().take(bound) {
      res = (res << 8) + byte as u16;
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a big endian unsigned 3 byte integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_u24;
///
/// let parser = |s| {
///   be_u24(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_u24<I, E: ParseError<I>>(input: I) -> IResult<I, u32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 3;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u32;
    for byte in input.iter_elements().take(bound) {
      res = (res << 8) + byte as u32;
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a big endian unsigned 4 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_u32;
///
/// let parser = |s| {
///   be_u32(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_u32<I, E: ParseError<I>>(input: I) -> IResult<I, u32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 4;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u32;
    for byte in input.iter_elements().take(bound) {
      res = (res << 8) + byte as u32;
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a big endian unsigned 8 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_u64;
///
/// let parser = |s| {
///   be_u64(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_u64<I, E: ParseError<I>>(input: I) -> IResult<I, u64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 8;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u64;
    for byte in input.iter_elements().take(bound) {
      res = (res << 8) + byte as u64;
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a big endian unsigned 16 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_u128;
///
/// let parser = |s| {
///   be_u128(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_u128<I, E: ParseError<I>>(input: I) -> IResult<I, u128, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 16;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u128;
    for byte in input.iter_elements().take(bound) {
      res = (res << 8) + byte as u128;
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a signed 1 byte integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_i8;
///
/// let parser = |s| {
///   be_i8(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
/// assert_eq!(parser(&b""[..]), Err(Err::Error((&[][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_i8<I, E: ParseError<I>>(input: I) -> IResult<I, i8, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  be_u8.map(|x| x as i8).parse(input)
}

/// Recognizes a big endian signed 2 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_i16;
///
/// let parser = |s| {
///   be_i16(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_i16<I, E: ParseError<I>>(input: I) -> IResult<I, i16, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  be_u16.map(|x| x as i16).parse(input)
}

/// Recognizes a big endian signed 3 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_i24;
///
/// let parser = |s| {
///   be_i24(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_i24<I, E: ParseError<I>>(input: I) -> IResult<I, i32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  // Same as the unsigned version but we need to sign-extend manually here
  be_u24
    .map(|x| {
      if x & 0x80_00_00 != 0 {
        (x | 0xff_00_00_00) as i32
      } else {
        x as i32
      }
    })
    .parse(input)
}

/// Recognizes a big endian signed 4 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_i32;
///
/// let parser = |s| {
///   be_i32(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_i32<I, E: ParseError<I>>(input: I) -> IResult<I, i32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  be_u32.map(|x| x as i32).parse(input)
}

/// Recognizes a big endian signed 8 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_i64;
///
/// let parser = |s| {
///   be_i64(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_i64<I, E: ParseError<I>>(input: I) -> IResult<I, i64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  be_u64.map(|x| x as i64).parse(input)
}

/// Recognizes a big endian signed 16 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_i128;
///
/// let parser = |s| {
///   be_i128(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_i128<I, E: ParseError<I>>(input: I) -> IResult<I, i128, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  be_u128.map(|x| x as i128).parse(input)
}

/// Recognizes an unsigned 1 byte integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_u8;
///
/// let parser = |s| {
///   le_u8(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
/// assert_eq!(parser(&b""[..]), Err(Err::Error((&[][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_u8<I, E: ParseError<I>>(input: I) -> IResult<I, u8, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 1;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let res = input.iter_elements().next().unwrap();

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a little endian unsigned 2 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_u16;
///
/// let parser = |s| {
///   le_u16(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_u16<I, E: ParseError<I>>(input: I) -> IResult<I, u16, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 2;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u16;
    for (index, byte) in input.iter_indices().take(bound) {
      res += (byte as u16) << (8 * index);
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a little endian unsigned 3 byte integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_u24;
///
/// let parser = |s| {
///   le_u24(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_u24<I, E: ParseError<I>>(input: I) -> IResult<I, u32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 3;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u32;
    for (index, byte) in input.iter_indices().take(bound) {
      res += (byte as u32) << (8 * index);
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a little endian unsigned 4 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_u32;
///
/// let parser = |s| {
///   le_u32(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_u32<I, E: ParseError<I>>(input: I) -> IResult<I, u32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 4;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u32;
    for (index, byte) in input.iter_indices().take(bound) {
      res += (byte as u32) << (8 * index);
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a little endian unsigned 8 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_u64;
///
/// let parser = |s| {
///   le_u64(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_u64<I, E: ParseError<I>>(input: I) -> IResult<I, u64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 8;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u64;
    for (index, byte) in input.iter_indices().take(bound) {
      res += (byte as u64) << (8 * index);
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a little endian unsigned 16 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_u128;
///
/// let parser = |s| {
///   le_u128(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_u128<I, E: ParseError<I>>(input: I) -> IResult<I, u128, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 16;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let mut res = 0u128;
    for (index, byte) in input.iter_indices().take(bound) {
      res += (byte as u128) << (8 * index);
    }

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes a signed 1 byte integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_i8;
///
/// let parser = |s| {
///   le_i8(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
/// assert_eq!(parser(&b""[..]), Err(Err::Error((&[][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_i8<I, E: ParseError<I>>(input: I) -> IResult<I, i8, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  be_u8.map(|x| x as i8).parse(input)
}

/// Recognizes a little endian signed 2 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_i16;
///
/// let parser = |s| {
///   le_i16(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_i16<I, E: ParseError<I>>(input: I) -> IResult<I, i16, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  le_u16.map(|x| x as i16).parse(input)
}

/// Recognizes a little endian signed 3 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_i24;
///
/// let parser = |s| {
///   le_i24(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_i24<I, E: ParseError<I>>(input: I) -> IResult<I, i32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  // Same as the unsigned version but we need to sign-extend manually here
  le_u24
    .map(|x| {
      if x & 0x80_00_00 != 0 {
        (x | 0xff_00_00_00) as i32
      } else {
        x as i32
      }
    })
    .parse(input)
}

/// Recognizes a little endian signed 4 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_i32;
///
/// let parser = |s| {
///   le_i32(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_i32<I, E: ParseError<I>>(input: I) -> IResult<I, i32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  le_u32.map(|x| x as i32).parse(input)
}

/// Recognizes a little endian signed 8 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_i64;
///
/// let parser = |s| {
///   le_i64(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_i64<I, E: ParseError<I>>(input: I) -> IResult<I, i64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  le_u64.map(|x| x as i64).parse(input)
}

/// Recognizes a little endian signed 16 bytes integer.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_i128;
///
/// let parser = |s| {
///   le_i128(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
/// assert_eq!(parser(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_i128<I, E: ParseError<I>>(input: I) -> IResult<I, i128, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  le_u128.map(|x| x as i128).parse(input)
}

/// Recognizes an unsigned 1 byte integer
///
/// Note that endianness does not apply to 1 byte numbers.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::u8;
///
/// let parser = |s| {
///   u8(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
/// assert_eq!(parser(&b""[..]), Err(Err::Error((&[][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn u8<I, E: ParseError<I>>(input: I) -> IResult<I, u8, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  let bound: usize = 1;
  if input.input_len() < bound {
    Err(Err::Error(make_error(input, ErrorKind::Eof)))
  } else {
    let res = input.iter_elements().next().unwrap();

    Ok((input.slice(bound..), res))
  }
}

/// Recognizes an unsigned 2 bytes integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian u16 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian u16 integer.
/// *complete version*: returns an error if there is not enough input data
///
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::u16;
///
/// let be_u16 = |s| {
///   u16(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_u16(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
/// assert_eq!(be_u16(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_u16 = |s| {
///   u16(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_u16(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
/// assert_eq!(le_u16(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn u16<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, u16, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_u16,
    crate::number::Endianness::Little => le_u16,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_u16,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_u16,
  }
}

/// Recognizes an unsigned 3 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian u24 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian u24 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::u24;
///
/// let be_u24 = |s| {
///   u24(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_u24(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
/// assert_eq!(be_u24(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_u24 = |s| {
///   u24(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_u24(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
/// assert_eq!(le_u24(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn u24<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, u32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_u24,
    crate::number::Endianness::Little => le_u24,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_u24,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_u24,
  }
}

/// Recognizes an unsigned 4 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian u32 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian u32 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::u32;
///
/// let be_u32 = |s| {
///   u32(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_u32(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
/// assert_eq!(be_u32(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_u32 = |s| {
///   u32(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_u32(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
/// assert_eq!(le_u32(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn u32<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, u32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_u32,
    crate::number::Endianness::Little => le_u32,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_u32,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_u32,
  }
}

/// Recognizes an unsigned 8 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian u64 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian u64 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::u64;
///
/// let be_u64 = |s| {
///   u64(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_u64(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
/// assert_eq!(be_u64(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_u64 = |s| {
///   u64(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_u64(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
/// assert_eq!(le_u64(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn u64<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, u64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_u64,
    crate::number::Endianness::Little => le_u64,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_u64,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_u64,
  }
}

/// Recognizes an unsigned 16 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian u128 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian u128 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::u128;
///
/// let be_u128 = |s| {
///   u128(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_u128(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
/// assert_eq!(be_u128(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_u128 = |s| {
///   u128(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_u128(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
/// assert_eq!(le_u128(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn u128<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, u128, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_u128,
    crate::number::Endianness::Little => le_u128,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_u128,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_u128,
  }
}

/// Recognizes a signed 1 byte integer
///
/// Note that endianness does not apply to 1 byte numbers.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::i8;
///
/// let parser = |s| {
///   i8(s)
/// };
///
/// assert_eq!(parser(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
/// assert_eq!(parser(&b""[..]), Err(Err::Error((&[][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn i8<I, E: ParseError<I>>(i: I) -> IResult<I, i8, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  u8.map(|x| x as i8).parse(i)
}

/// Recognizes a signed 2 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian i16 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian i16 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::i16;
///
/// let be_i16 = |s| {
///   i16(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_i16(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
/// assert_eq!(be_i16(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_i16 = |s| {
///   i16(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_i16(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
/// assert_eq!(le_i16(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn i16<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, i16, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_i16,
    crate::number::Endianness::Little => le_i16,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_i16,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_i16,
  }
}

/// Recognizes a signed 3 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian i24 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian i24 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::i24;
///
/// let be_i24 = |s| {
///   i24(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_i24(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
/// assert_eq!(be_i24(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_i24 = |s| {
///   i24(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_i24(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
/// assert_eq!(le_i24(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn i24<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, i32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_i24,
    crate::number::Endianness::Little => le_i24,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_i24,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_i24,
  }
}

/// Recognizes a signed 4 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian i32 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian i32 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::i32;
///
/// let be_i32 = |s| {
///   i32(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_i32(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
/// assert_eq!(be_i32(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_i32 = |s| {
///   i32(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_i32(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
/// assert_eq!(le_i32(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn i32<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, i32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_i32,
    crate::number::Endianness::Little => le_i32,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_i32,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_i32,
  }
}

/// Recognizes a signed 8 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian i64 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian i64 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::i64;
///
/// let be_i64 = |s| {
///   i64(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_i64(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
/// assert_eq!(be_i64(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_i64 = |s| {
///   i64(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_i64(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
/// assert_eq!(le_i64(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn i64<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, i64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_i64,
    crate::number::Endianness::Little => le_i64,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_i64,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_i64,
  }
}

/// Recognizes a signed 16 byte integer
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian i128 integer,
/// otherwise if `nom::number::Endianness::Little` parse a little endian i128 integer.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::i128;
///
/// let be_i128 = |s| {
///   i128(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_i128(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
/// assert_eq!(be_i128(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
///
/// let le_i128 = |s| {
///   i128(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_i128(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
/// assert_eq!(le_i128(&b"\x01"[..]), Err(Err::Error((&[0x01][..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn i128<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, i128, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_i128,
    crate::number::Endianness::Little => le_i128,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_i128,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_i128,
  }
}

/// Recognizes a big endian 4 bytes floating point number.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_f32;
///
/// let parser = |s| {
///   be_f32(s)
/// };
///
/// assert_eq!(parser(&[0x41, 0x48, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(parser(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_f32<I, E: ParseError<I>>(input: I) -> IResult<I, f32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match be_u32(input) {
    Err(e) => Err(e),
    Ok((i, o)) => Ok((i, f32::from_bits(o))),
  }
}

/// Recognizes a big endian 8 bytes floating point number.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::be_f64;
///
/// let parser = |s| {
///   be_f64(s)
/// };
///
/// assert_eq!(parser(&[0x40, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(parser(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn be_f64<I, E: ParseError<I>>(input: I) -> IResult<I, f64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match be_u64(input) {
    Err(e) => Err(e),
    Ok((i, o)) => Ok((i, f64::from_bits(o))),
  }
}

/// Recognizes a little endian 4 bytes floating point number.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_f32;
///
/// let parser = |s| {
///   le_f32(s)
/// };
///
/// assert_eq!(parser(&[0x00, 0x00, 0x48, 0x41][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(parser(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_f32<I, E: ParseError<I>>(input: I) -> IResult<I, f32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match le_u32(input) {
    Err(e) => Err(e),
    Ok((i, o)) => Ok((i, f32::from_bits(o))),
  }
}

/// Recognizes a little endian 8 bytes floating point number.
///
/// *Complete version*: Returns an error if there is not enough input data.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::le_f64;
///
/// let parser = |s| {
///   le_f64(s)
/// };
///
/// assert_eq!(parser(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x40][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(parser(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn le_f64<I, E: ParseError<I>>(input: I) -> IResult<I, f64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match le_u64(input) {
    Err(e) => Err(e),
    Ok((i, o)) => Ok((i, f64::from_bits(o))),
  }
}

/// Recognizes a 4 byte floating point number
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian f32 float,
/// otherwise if `nom::number::Endianness::Little` parse a little endian f32 float.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::f32;
///
/// let be_f32 = |s| {
///   f32(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_f32(&[0x41, 0x48, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(be_f32(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
///
/// let le_f32 = |s| {
///   f32(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_f32(&[0x00, 0x00, 0x48, 0x41][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(le_f32(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn f32<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, f32, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_f32,
    crate::number::Endianness::Little => le_f32,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_f32,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_f32,
  }
}

/// Recognizes an 8 byte floating point number
///
/// If the parameter is `nom::number::Endianness::Big`, parse a big endian f64 float,
/// otherwise if `nom::number::Endianness::Little` parse a little endian f64 float.
/// *complete version*: returns an error if there is not enough input data
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::f64;
///
/// let be_f64 = |s| {
///   f64(nom::number::Endianness::Big)(s)
/// };
///
/// assert_eq!(be_f64(&[0x40, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(be_f64(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
///
/// let le_f64 = |s| {
///   f64(nom::number::Endianness::Little)(s)
/// };
///
/// assert_eq!(le_f64(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x40][..]), Ok((&b""[..], 12.5)));
/// assert_eq!(le_f64(&b"abc"[..]), Err(Err::Error((&b"abc"[..], ErrorKind::Eof))));
/// ```
#[inline]
pub fn f64<I, E: ParseError<I>>(endian: crate::number::Endianness) -> fn(I) -> IResult<I, f64, E>
where
  I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
{
  match endian {
    crate::number::Endianness::Big => be_f64,
    crate::number::Endianness::Little => le_f64,
    #[cfg(target_endian = "big")]
    crate::number::Endianness::Native => be_f64,
    #[cfg(target_endian = "little")]
    crate::number::Endianness::Native => le_f64,
  }
}

/// Recognizes a hex-encoded integer.
///
/// *Complete version*: Will parse until the end of input if it has less than 8 bytes.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::hex_u32;
///
/// let parser = |s| {
///   hex_u32(s)
/// };
///
/// assert_eq!(parser(&b"01AE"[..]), Ok((&b""[..], 0x01AE)));
/// assert_eq!(parser(&b"abc"[..]), Ok((&b""[..], 0x0ABC)));
/// assert_eq!(parser(&b"ggg"[..]), Err(Err::Error((&b"ggg"[..], ErrorKind::IsA))));
/// ```
#[inline]
pub fn hex_u32<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], u32, E> {
  let (i, o) = crate::bytes::complete::is_a(&b"0123456789abcdefABCDEF"[..])(input)?;
  // Do not parse more than 8 characters for a u32
  let (parsed, remaining) = if o.len() <= 8 {
    (o, i)
  } else {
    (&input[..8], &input[8..])
  };

  let res = parsed
    .iter()
    .rev()
    .enumerate()
    .map(|(k, &v)| {
      let digit = v as char;
      digit.to_digit(16).unwrap_or(0) << (k * 4)
    })
    .sum();

  Ok((remaining, res))
}

/// Recognizes floating point number in a byte string and returns the corresponding slice.
///
/// *Complete version*: Can parse until the end of input.
///
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::recognize_float;
///
/// let parser = |s| {
///   recognize_float(s)
/// };
///
/// assert_eq!(parser("11e-1"), Ok(("", "11e-1")));
/// assert_eq!(parser("123E-02"), Ok(("", "123E-02")));
/// assert_eq!(parser("123K-01"), Ok(("K-01", "123")));
/// assert_eq!(parser("abc"), Err(Err::Error(("abc", ErrorKind::Char))));
/// ```
#[rustfmt::skip]
pub fn recognize_float<T, E:ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: Clone + Offset,
  T: InputIter,
  <T as InputIter>::Item: AsChar,
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  recognize(
    tuple((
      opt(alt((char('+'), char('-')))),
      alt((
        map(tuple((digit1, opt(pair(char('.'), opt(digit1))))), |_| ()),
        map(tuple((char('.'), digit1)), |_| ())
      )),
      opt(tuple((
        alt((char('e'), char('E'))),
        opt(alt((char('+'), char('-')))),
        cut(digit1)
      )))
    ))
  )(input)
}

// workaround until issues with minimal-lexical are fixed
#[doc(hidden)]
pub fn recognize_float_or_exceptions<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: Clone + Offset,
  T: InputIter + InputTake + Compare<&'static str>,
  <T as InputIter>::Item: AsChar,
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  alt((
    |i: T| {
      recognize_float::<_, E>(i.clone()).map_err(|e| match e {
        crate::Err::Error(_) => crate::Err::Error(E::from_error_kind(i, ErrorKind::Float)),
        crate::Err::Failure(_) => crate::Err::Failure(E::from_error_kind(i, ErrorKind::Float)),
        crate::Err::Incomplete(needed) => crate::Err::Incomplete(needed),
      })
    },
    |i: T| {
      crate::bytes::complete::tag_no_case::<_, _, E>("nan")(i.clone())
        .map_err(|_| crate::Err::Error(E::from_error_kind(i, ErrorKind::Float)))
    },
    |i: T| {
      crate::bytes::complete::tag_no_case::<_, _, E>("inf")(i.clone())
        .map_err(|_| crate::Err::Error(E::from_error_kind(i, ErrorKind::Float)))
    },
    |i: T| {
      crate::bytes::complete::tag_no_case::<_, _, E>("infinity")(i.clone())
        .map_err(|_| crate::Err::Error(E::from_error_kind(i, ErrorKind::Float)))
    },
  ))(input)
}

/// Recognizes a floating point number in text format
///
/// It returns a tuple of (`sign`, `integer part`, `fraction part` and `exponent`) of the input
/// data.
///
/// *Complete version*: Can parse until the end of input.
///
pub fn recognize_float_parts<T, E: ParseError<T>>(input: T) -> IResult<T, (bool, T, T, i32), E>
where
  T: Slice<RangeFrom<usize>> + Slice<RangeTo<usize>> + Slice<Range<usize>>,
  T: Clone + Offset,
  T: InputIter + InputTake,
  <T as InputIter>::Item: AsChar + Copy,
  T: InputTakeAtPosition + InputLength,
  <T as InputTakeAtPosition>::Item: AsChar,
  T: for<'a> Compare<&'a [u8]>,
  T: AsBytes,
{
  let (i, sign) = sign(input.clone())?;

  //let (i, zeroes) = take_while(|c: <T as InputTakeAtPosition>::Item| c.as_char() == '0')(i)?;
  let (i, zeroes) = match i.as_bytes().iter().position(|c| *c != b'0') {
    Some(index) => i.take_split(index),
    None => i.take_split(i.input_len()),
  };
  //let (i, mut integer) = digit0(i)?;
  let (i, mut integer) = match i
    .as_bytes()
    .iter()
    .position(|c| !(*c >= b'0' && *c <= b'9'))
  {
    Some(index) => i.take_split(index),
    None => i.take_split(i.input_len()),
  };

  if integer.input_len() == 0 && zeroes.input_len() > 0 {
    // keep the last zero if integer is empty
    integer = zeroes.slice(zeroes.input_len() - 1..);
  }

  let (i, opt_dot) = opt(tag(&b"."[..]))(i)?;
  let (i, fraction) = if opt_dot.is_none() {
    let i2 = i.clone();
    (i2, i.slice(..0))
  } else {
    // match number, trim right zeroes
    let mut zero_count = 0usize;
    let mut position = None;
    for (pos, c) in i.as_bytes().iter().enumerate() {
      if *c >= b'0' && *c <= b'9' {
        if *c == b'0' {
          zero_count += 1;
        } else {
          zero_count = 0;
        }
      } else {
        position = Some(pos);
        break;
      }
    }

    let position = position.unwrap_or(i.input_len());

    let index = if zero_count == 0 {
      position
    } else if zero_count == position {
      position - zero_count + 1
    } else {
      position - zero_count
    };

    (i.slice(position..), i.slice(..index))
  };

  if integer.input_len() == 0 && fraction.input_len() == 0 {
    return Err(Err::Error(E::from_error_kind(input, ErrorKind::Float)));
  }

  let i2 = i.clone();
  let (i, e) = match i.as_bytes().iter().next() {
    Some(b'e') => (i.slice(1..), true),
    Some(b'E') => (i.slice(1..), true),
    _ => (i, false),
  };

  let (i, exp) = if e {
    cut(crate::character::complete::i32)(i)?
  } else {
    (i2, 0)
  };

  Ok((i, (sign, integer, fraction, exp)))
}

use crate::traits::ParseTo;

/// Recognizes floating point number in text format and returns a f32.
///
/// *Complete version*: Can parse until the end of input.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::float;
///
/// let parser = |s| {
///   float(s)
/// };
///
/// assert_eq!(parser("11e-1"), Ok(("", 1.1)));
/// assert_eq!(parser("123E-02"), Ok(("", 1.23)));
/// assert_eq!(parser("123K-01"), Ok(("K-01", 123.0)));
/// assert_eq!(parser("abc"), Err(Err::Error(("abc", ErrorKind::Float))));
/// ```
pub fn float<T, E: ParseError<T>>(input: T) -> IResult<T, f32, E>
where
  T: Slice<RangeFrom<usize>> + Slice<RangeTo<usize>> + Slice<Range<usize>>,
  T: Clone + Offset + ParseTo<f32> + Compare<&'static str>,
  T: InputIter + InputLength + InputTake,
  <T as InputIter>::Item: AsChar + Copy,
  <T as InputIter>::IterElem: Clone,
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
  T: AsBytes,
  T: for<'a> Compare<&'a [u8]>,
{
  /*
  let (i, (sign, integer, fraction, exponent)) = recognize_float_parts(input)?;

  let mut float: f32 = minimal_lexical::parse_float(
    integer.as_bytes().iter(),
    fraction.as_bytes().iter(),
    exponent,
  );
  if !sign {
    float = -float;
  }

  Ok((i, float))
      */
  let (i, s) = recognize_float_or_exceptions(input)?;
  match s.parse_to() {
    Some(f) => Ok((i, f)),
    None => Err(crate::Err::Error(E::from_error_kind(
      i,
      crate::error::ErrorKind::Float,
    ))),
  }
}

/// Recognizes floating point number in text format and returns a f64.
///
/// *Complete version*: Can parse until the end of input.
/// ```rust
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::Needed::Size;
/// use nom::number::complete::double;
///
/// let parser = |s| {
///   double(s)
/// };
///
/// assert_eq!(parser("11e-1"), Ok(("", 1.1)));
/// assert_eq!(parser("123E-02"), Ok(("", 1.23)));
/// assert_eq!(parser("123K-01"), Ok(("K-01", 123.0)));
/// assert_eq!(parser("abc"), Err(Err::Error(("abc", ErrorKind::Float))));
/// ```
pub fn double<T, E: ParseError<T>>(input: T) -> IResult<T, f64, E>
where
  T: Slice<RangeFrom<usize>> + Slice<RangeTo<usize>> + Slice<Range<usize>>,
  T: Clone + Offset + ParseTo<f64> + Compare<&'static str>,
  T: InputIter + InputLength + InputTake,
  <T as InputIter>::Item: AsChar + Copy,
  <T as InputIter>::IterElem: Clone,
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
  T: AsBytes,
  T: for<'a> Compare<&'a [u8]>,
{
  /*
  let (i, (sign, integer, fraction, exponent)) = recognize_float_parts(input)?;

  let mut float: f64 = minimal_lexical::parse_float(
    integer.as_bytes().iter(),
    fraction.as_bytes().iter(),
    exponent,
  );
  if !sign {
    float = -float;
  }

  Ok((i, float))
      */
  let (i, s) = recognize_float_or_exceptions(input)?;
  match s.parse_to() {
    Some(f) => Ok((i, f)),
    None => Err(crate::Err::Error(E::from_error_kind(
      i,
      crate::error::ErrorKind::Float,
    ))),
  }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/nom/unit/src/number/complete/tests_1647.rs"));
