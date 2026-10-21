//! Character specific parsers and combinators, streaming version
//!
//! Functions recognizing specific characters

use crate::branch::alt;
use crate::combinator::opt;
use crate::error::ErrorKind;
use crate::error::ParseError;
use crate::internal::{Err, IResult, Needed};
use crate::lib::std::ops::{Range, RangeFrom, RangeTo};
use crate::traits::{
  AsChar, FindToken, InputIter, InputLength, InputTake, InputTakeAtPosition, Slice,
};
use crate::traits::{Compare, CompareResult};

/// Recognizes one character.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{ErrorKind, Error}, Needed, IResult};
/// # use nom::character::streaming::char;
/// fn parser(i: &str) -> IResult<&str, char> {
///     char('a')(i)
/// }
/// assert_eq!(parser("abc"), Ok(("bc", 'a')));
/// assert_eq!(parser("bc"), Err(Err::Error(Error::new("bc", ErrorKind::Char))));
/// assert_eq!(parser(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn char<I, Error: ParseError<I>>(c: char) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter + InputLength,
  <I as InputIter>::Item: AsChar,
{
  move |i: I| match (i).iter_elements().next().map(|t| {
    let b = t.as_char() == c;
    (&c, b)
  }) {
    None => Err(Err::Incomplete(Needed::new(c.len() - i.input_len()))),
    Some((_, false)) => Err(Err::Error(Error::from_char(i, c))),
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
  }
}

/// Recognizes one character and checks that it satisfies a predicate
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{ErrorKind, Error}, Needed, IResult};
/// # use nom::character::streaming::satisfy;
/// fn parser(i: &str) -> IResult<&str, char> {
///     satisfy(|c| c == 'a' || c == 'b')(i)
/// }
/// assert_eq!(parser("abc"), Ok(("bc", 'a')));
/// assert_eq!(parser("cd"), Err(Err::Error(Error::new("cd", ErrorKind::Satisfy))));
/// assert_eq!(parser(""), Err(Err::Incomplete(Needed::Unknown)));
/// ```
pub fn satisfy<F, I, Error: ParseError<I>>(cond: F) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar,
  F: Fn(char) -> bool,
{
  move |i: I| match (i).iter_elements().next().map(|t| {
    let c = t.as_char();
    let b = cond(c);
    (c, b)
  }) {
    None => Err(Err::Incomplete(Needed::Unknown)),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::Satisfy))),
    Some((c, true)) => Ok((i.slice(c.len()..), c)),
  }
}

/// Recognizes one of the provided characters.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::character::streaming::one_of;
/// assert_eq!(one_of::<_, _, (_, ErrorKind)>("abc")("b"), Ok(("", 'b')));
/// assert_eq!(one_of::<_, _, (_, ErrorKind)>("a")("bc"), Err(Err::Error(("bc", ErrorKind::OneOf))));
/// assert_eq!(one_of::<_, _, (_, ErrorKind)>("a")(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn one_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar + Copy,
  T: FindToken<<I as InputIter>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, list.find_token(c))) {
    None => Err(Err::Incomplete(Needed::new(1))),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::OneOf))),
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
  }
}

/// Recognizes a character that is not in the provided characters.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, Needed};
/// # use nom::character::streaming::none_of;
/// assert_eq!(none_of::<_, _, (_, ErrorKind)>("abc")("z"), Ok(("", 'z')));
/// assert_eq!(none_of::<_, _, (_, ErrorKind)>("ab")("a"), Err(Err::Error(("a", ErrorKind::NoneOf))));
/// assert_eq!(none_of::<_, _, (_, ErrorKind)>("a")(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn none_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar + Copy,
  T: FindToken<<I as InputIter>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, !list.find_token(c))) {
    None => Err(Err::Incomplete(Needed::new(1))),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::NoneOf))),
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
  }
}

/// Recognizes the string "\r\n".
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::crlf;
/// assert_eq!(crlf::<_, (_, ErrorKind)>("\r\nc"), Ok(("c", "\r\n")));
/// assert_eq!(crlf::<_, (_, ErrorKind)>("ab\r\nc"), Err(Err::Error(("ab\r\nc", ErrorKind::CrLf))));
/// assert_eq!(crlf::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(2))));
/// ```
pub fn crlf<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter,
  T: Compare<&'static str>,
{
  match input.compare("\r\n") {
    //FIXME: is this the right index?
    CompareResult::Ok => Ok((input.slice(2..), input.slice(0..2))),
    CompareResult::Incomplete => Err(Err::Incomplete(Needed::new(2))),
    CompareResult::Error => {
      let e: ErrorKind = ErrorKind::CrLf;
      Err(Err::Error(E::from_error_kind(input, e)))
    }
  }
}

/// Recognizes a string of any char except '\r\n' or '\n'.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::streaming::not_line_ending;
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("ab\r\nc"), Ok(("\r\nc", "ab")));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("abc"), Err(Err::Incomplete(Needed::Unknown)));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::Unknown)));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("a\rb\nc"), Err(Err::Error(("a\rb\nc", ErrorKind::Tag ))));
/// assert_eq!(not_line_ending::<_, (_, ErrorKind)>("a\rbc"), Err(Err::Error(("a\rbc", ErrorKind::Tag ))));
/// ```
pub fn not_line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + InputLength,
  T: Compare<&'static str>,
  <T as InputIter>::Item: AsChar,
  <T as InputIter>::Item: AsChar,
{
  match input.position(|item| {
    let c = item.as_char();
    c == '\r' || c == '\n'
  }) {
    None => Err(Err::Incomplete(Needed::Unknown)),
    Some(index) => {
      let mut it = input.slice(index..).iter_elements();
      let nth = it.next().unwrap().as_char();
      if nth == '\r' {
        let sliced = input.slice(index..);
        let comp = sliced.compare("\r\n");
        match comp {
          //FIXME: calculate the right index
          CompareResult::Incomplete => Err(Err::Incomplete(Needed::Unknown)),
          CompareResult::Error => {
            let e: ErrorKind = ErrorKind::Tag;
            Err(Err::Error(E::from_error_kind(input, e)))
          }
          CompareResult::Ok => Ok((input.slice(index..), input.slice(..index))),
        }
      } else {
        Ok((input.slice(index..), input.slice(..index)))
      }
    }
  }
}

/// Recognizes an end of line (both '\n' and '\r\n').
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::line_ending;
/// assert_eq!(line_ending::<_, (_, ErrorKind)>("\r\nc"), Ok(("c", "\r\n")));
/// assert_eq!(line_ending::<_, (_, ErrorKind)>("ab\r\nc"), Err(Err::Error(("ab\r\nc", ErrorKind::CrLf))));
/// assert_eq!(line_ending::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + InputLength,
  T: Compare<&'static str>,
{
  match input.compare("\n") {
    CompareResult::Ok => Ok((input.slice(1..), input.slice(0..1))),
    CompareResult::Incomplete => Err(Err::Incomplete(Needed::new(1))),
    CompareResult::Error => {
      match input.compare("\r\n") {
        //FIXME: is this the right index?
        CompareResult::Ok => Ok((input.slice(2..), input.slice(0..2))),
        CompareResult::Incomplete => Err(Err::Incomplete(Needed::new(2))),
        CompareResult::Error => Err(Err::Error(E::from_error_kind(input, ErrorKind::CrLf))),
      }
    }
  }
}

/// Matches a newline character '\\n'.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::newline;
/// assert_eq!(newline::<_, (_, ErrorKind)>("\nc"), Ok(("c", '\n')));
/// assert_eq!(newline::<_, (_, ErrorKind)>("\r\nc"), Err(Err::Error(("\r\nc", ErrorKind::Char))));
/// assert_eq!(newline::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn newline<I, Error: ParseError<I>>(input: I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter + InputLength,
  <I as InputIter>::Item: AsChar,
{
  char('\n')(input)
}

/// Matches a tab character '\t'.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::tab;
/// assert_eq!(tab::<_, (_, ErrorKind)>("\tc"), Ok(("c", '\t')));
/// assert_eq!(tab::<_, (_, ErrorKind)>("\r\nc"), Err(Err::Error(("\r\nc", ErrorKind::Char))));
/// assert_eq!(tab::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn tab<I, Error: ParseError<I>>(input: I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter + InputLength,
  <I as InputIter>::Item: AsChar,
{
  char('\t')(input)
}

/// Matches one byte as a character. Note that the input type will
/// accept a `str`, but not a `&[u8]`, unlike many other nom parsers.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{character::streaming::anychar, Err, error::ErrorKind, IResult, Needed};
/// assert_eq!(anychar::<_, (_, ErrorKind)>("abc"), Ok(("bc",'a')));
/// assert_eq!(anychar::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn anychar<T, E: ParseError<T>>(input: T) -> IResult<T, char, E>
where
  T: InputIter + InputLength + Slice<RangeFrom<usize>>,
  <T as InputIter>::Item: AsChar,
{
  let mut it = input.iter_indices();
  match it.next() {
    None => Err(Err::Incomplete(Needed::new(1))),
    Some((_, c)) => match it.next() {
      None => Ok((input.slice(input.input_len()..), c.as_char())),
      Some((idx, _)) => Ok((input.slice(idx..), c.as_char())),
    },
  }
}

/// Recognizes zero or more lowercase and uppercase ASCII alphabetic characters: a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphabetic character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alpha0;
/// assert_eq!(alpha0::<_, (_, ErrorKind)>("ab1c"), Ok(("1c", "ab")));
/// assert_eq!(alpha0::<_, (_, ErrorKind)>("1c"), Ok(("1c", "")));
/// assert_eq!(alpha0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alpha0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_alpha())
}

/// Recognizes one or more lowercase and uppercase ASCII alphabetic characters: a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphabetic character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alpha1;
/// assert_eq!(alpha1::<_, (_, ErrorKind)>("aB1c"), Ok(("1c", "aB")));
/// assert_eq!(alpha1::<_, (_, ErrorKind)>("1c"), Err(Err::Error(("1c", ErrorKind::Alpha))));
/// assert_eq!(alpha1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alpha1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_alpha(), ErrorKind::Alpha)
}

/// Recognizes zero or more ASCII numerical characters: 0-9
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::digit0;
/// assert_eq!(digit0::<_, (_, ErrorKind)>("21c"), Ok(("c", "21")));
/// assert_eq!(digit0::<_, (_, ErrorKind)>("a21c"), Ok(("a21c", "")));
/// assert_eq!(digit0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_dec_digit())
}

/// Recognizes one or more ASCII numerical characters: 0-9
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::digit1;
/// assert_eq!(digit1::<_, (_, ErrorKind)>("21c"), Ok(("c", "21")));
/// assert_eq!(digit1::<_, (_, ErrorKind)>("c1"), Err(Err::Error(("c1", ErrorKind::Digit))));
/// assert_eq!(digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_dec_digit(), ErrorKind::Digit)
}

/// Recognizes zero or more ASCII hexadecimal numerical characters: 0-9, A-F, a-f
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non hexadecimal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::hex_digit0;
/// assert_eq!(hex_digit0::<_, (_, ErrorKind)>("21cZ"), Ok(("Z", "21c")));
/// assert_eq!(hex_digit0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(hex_digit0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn hex_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_hex_digit())
}

/// Recognizes one or more ASCII hexadecimal numerical characters: 0-9, A-F, a-f
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non hexadecimal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::hex_digit1;
/// assert_eq!(hex_digit1::<_, (_, ErrorKind)>("21cZ"), Ok(("Z", "21c")));
/// assert_eq!(hex_digit1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::HexDigit))));
/// assert_eq!(hex_digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn hex_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_hex_digit(), ErrorKind::HexDigit)
}

/// Recognizes zero or more octal characters: 0-7
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non octal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::oct_digit0;
/// assert_eq!(oct_digit0::<_, (_, ErrorKind)>("21cZ"), Ok(("cZ", "21")));
/// assert_eq!(oct_digit0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(oct_digit0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn oct_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_oct_digit())
}

/// Recognizes one or more octal characters: 0-7
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non octal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::oct_digit1;
/// assert_eq!(oct_digit1::<_, (_, ErrorKind)>("21cZ"), Ok(("cZ", "21")));
/// assert_eq!(oct_digit1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::OctDigit))));
/// assert_eq!(oct_digit1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn oct_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_oct_digit(), ErrorKind::OctDigit)
}

/// Recognizes zero or more ASCII numerical and alphabetic characters: 0-9, a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphanumerical character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alphanumeric0;
/// assert_eq!(alphanumeric0::<_, (_, ErrorKind)>("21cZ%1"), Ok(("%1", "21cZ")));
/// assert_eq!(alphanumeric0::<_, (_, ErrorKind)>("&Z21c"), Ok(("&Z21c", "")));
/// assert_eq!(alphanumeric0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alphanumeric0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_alphanum())
}

/// Recognizes one or more ASCII numerical and alphabetic characters: 0-9, a-z, A-Z
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non alphanumerical character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::alphanumeric1;
/// assert_eq!(alphanumeric1::<_, (_, ErrorKind)>("21cZ%1"), Ok(("%1", "21cZ")));
/// assert_eq!(alphanumeric1::<_, (_, ErrorKind)>("&H2"), Err(Err::Error(("&H2", ErrorKind::AlphaNumeric))));
/// assert_eq!(alphanumeric1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn alphanumeric1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_alphanum(), ErrorKind::AlphaNumeric)
}

/// Recognizes zero or more spaces and tabs.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::space0;
/// assert_eq!(space0::<_, (_, ErrorKind)>(" \t21c"), Ok(("21c", " \t")));
/// assert_eq!(space0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(space0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn space0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position(|item| {
    let c = item.as_char();
    !(c == ' ' || c == '\t')
  })
}
/// Recognizes one or more spaces and tabs.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::space1;
/// assert_eq!(space1::<_, (_, ErrorKind)>(" \t21c"), Ok(("21c", " \t")));
/// assert_eq!(space1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::Space))));
/// assert_eq!(space1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn space1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position1(
    |item| {
      let c = item.as_char();
      !(c == ' ' || c == '\t')
    },
    ErrorKind::Space,
  )
}

/// Recognizes zero or more spaces, tabs, carriage returns and line feeds.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::multispace0;
/// assert_eq!(multispace0::<_, (_, ErrorKind)>(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
/// assert_eq!(multispace0::<_, (_, ErrorKind)>("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(multispace0::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn multispace0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position(|item| {
    let c = item.as_char();
    !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
  })
}

/// Recognizes one or more spaces, tabs, carriage returns and line feeds.
///
/// *Streaming version*: Will return `Err(nom::Err::Incomplete(_))` if there's not enough input data,
/// or if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::streaming::multispace1;
/// assert_eq!(multispace1::<_, (_, ErrorKind)>(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
/// assert_eq!(multispace1::<_, (_, ErrorKind)>("H2"), Err(Err::Error(("H2", ErrorKind::MultiSpace))));
/// assert_eq!(multispace1::<_, (_, ErrorKind)>(""), Err(Err::Incomplete(Needed::new(1))));
/// ```
pub fn multispace1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position1(
    |item| {
      let c = item.as_char();
      !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
    },
    ErrorKind::MultiSpace,
  )
}

pub(crate) fn sign<T, E: ParseError<T>>(input: T) -> IResult<T, bool, E>
where
  T: Clone + InputTake + InputLength,
  T: for<'a> Compare<&'a [u8]>,
{
  use crate::bytes::streaming::tag;
  use crate::combinator::value;

  let (i, opt_sign) = opt(alt((
    value(false, tag(&b"-"[..])),
    value(true, tag(&b"+"[..])),
  )))(input)?;
  let sign = opt_sign.unwrap_or(true);

  Ok((i, sign))
}

#[doc(hidden)]
macro_rules! ints {
    ($($t:tt)+) => {
        $(
        /// will parse a number in text form to a number
        ///
        /// *Complete version*: can parse until the end of input.
        pub fn $t<T, E: ParseError<T>>(input: T) -> IResult<T, $t, E>
            where
            T: InputIter + Slice<RangeFrom<usize>> + InputLength + InputTake + Clone,
            <T as InputIter>::Item: AsChar,
            T: for <'a> Compare<&'a[u8]>,
            {
              let (i, sign) = sign(input.clone())?;

                if i.input_len() == 0 {
                    return Err(Err::Incomplete(Needed::new(1)));
                }

                let mut value: $t = 0;
                if sign {
                    for (pos, c) in i.iter_indices() {
                        match c.as_char().to_digit(10) {
                            None => {
                                if pos == 0 {
                                    return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit)));
                                } else {
                                    return Ok((i.slice(pos..), value));
                                }
                            },
                            Some(d) => match value.checked_mul(10).and_then(|v| v.checked_add(d as $t)) {
                                None => return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit))),
                                Some(v) => value = v,
                            }
                        }
                    }
                } else {
                    for (pos, c) in i.iter_indices() {
                        match c.as_char().to_digit(10) {
                            None => {
                                if pos == 0 {
                                    return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit)));
                                } else {
                                    return Ok((i.slice(pos..), value));
                                }
                            },
                            Some(d) => match value.checked_mul(10).and_then(|v| v.checked_sub(d as $t)) {
                                None => return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit))),
                                Some(v) => value = v,
                            }
                        }
                    }
                }

                Err(Err::Incomplete(Needed::new(1)))
            }
        )+
    }
}

ints! { i8 i16 i32 i64 i128 }

#[doc(hidden)]
macro_rules! uints {
    ($($t:tt)+) => {
        $(
        /// will parse a number in text form to a number
        ///
        /// *Complete version*: can parse until the end of input.
        pub fn $t<T, E: ParseError<T>>(input: T) -> IResult<T, $t, E>
            where
            T: InputIter + Slice<RangeFrom<usize>> + InputLength,
            <T as InputIter>::Item: AsChar,
            {
                let i = input;

                if i.input_len() == 0 {
                    return Err(Err::Incomplete(Needed::new(1)));
                }

                let mut value: $t = 0;
                for (pos, c) in i.iter_indices() {
                    match c.as_char().to_digit(10) {
                        None => {
                            if pos == 0 {
                                return Err(Err::Error(E::from_error_kind(i, ErrorKind::Digit)));
                            } else {
                                return Ok((i.slice(pos..), value));
                            }
                        },
                        Some(d) => match value.checked_mul(10).and_then(|v| v.checked_add(d as $t)) {
                            None => return Err(Err::Error(E::from_error_kind(i, ErrorKind::Digit))),
                            Some(v) => value = v,
                        }
                    }
                }

                Err(Err::Incomplete(Needed::new(1)))
            }
        )+
    }
}

uints! { u8 u16 u32 u64 u128 }

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/nom/unit/src/character/streaming/tests_737.rs"));
