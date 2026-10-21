//! Character specific parsers and combinators, complete input version.
//!
//! Functions recognizing specific characters.

use crate::branch::alt;
use crate::combinator::opt;
use crate::error::ErrorKind;
use crate::error::ParseError;
use crate::internal::{Err, IResult};
use crate::lib::std::ops::{Range, RangeFrom, RangeTo};
use crate::traits::{
  AsChar, FindToken, InputIter, InputLength, InputTake, InputTakeAtPosition, Slice,
};
use crate::traits::{Compare, CompareResult};

/// Recognizes one character.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{ErrorKind, Error}, IResult};
/// # use nom::character::complete::char;
/// fn parser(i: &str) -> IResult<&str, char> {
///     char('a')(i)
/// }
/// assert_eq!(parser("abc"), Ok(("bc", 'a')));
/// assert_eq!(parser(" abc"), Err(Err::Error(Error::new(" abc", ErrorKind::Char))));
/// assert_eq!(parser("bc"), Err(Err::Error(Error::new("bc", ErrorKind::Char))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Char))));
/// ```
pub fn char<I, Error: ParseError<I>>(c: char) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar,
{
  move |i: I| match (i).iter_elements().next().map(|t| {
    let b = t.as_char() == c;
    (&c, b)
  }) {
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
    _ => Err(Err::Error(Error::from_char(i, c))),
  }
}

/// Recognizes one character and checks that it satisfies a predicate
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{ErrorKind, Error}, Needed, IResult};
/// # use nom::character::complete::satisfy;
/// fn parser(i: &str) -> IResult<&str, char> {
///     satisfy(|c| c == 'a' || c == 'b')(i)
/// }
/// assert_eq!(parser("abc"), Ok(("bc", 'a')));
/// assert_eq!(parser("cd"), Err(Err::Error(Error::new("cd", ErrorKind::Satisfy))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Satisfy))));
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
    Some((c, true)) => Ok((i.slice(c.len()..), c)),
    _ => Err(Err::Error(Error::from_error_kind(i, ErrorKind::Satisfy))),
  }
}

/// Recognizes one of the provided characters.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind};
/// # use nom::character::complete::one_of;
/// assert_eq!(one_of::<_, _, (&str, ErrorKind)>("abc")("b"), Ok(("", 'b')));
/// assert_eq!(one_of::<_, _, (&str, ErrorKind)>("a")("bc"), Err(Err::Error(("bc", ErrorKind::OneOf))));
/// assert_eq!(one_of::<_, _, (&str, ErrorKind)>("a")(""), Err(Err::Error(("", ErrorKind::OneOf))));
/// ```
pub fn one_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar + Copy,
  T: FindToken<<I as InputIter>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, list.find_token(c))) {
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
    _ => Err(Err::Error(Error::from_error_kind(i, ErrorKind::OneOf))),
  }
}

/// Recognizes a character that is not in the provided characters.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind};
/// # use nom::character::complete::none_of;
/// assert_eq!(none_of::<_, _, (&str, ErrorKind)>("abc")("z"), Ok(("", 'z')));
/// assert_eq!(none_of::<_, _, (&str, ErrorKind)>("ab")("a"), Err(Err::Error(("a", ErrorKind::NoneOf))));
/// assert_eq!(none_of::<_, _, (&str, ErrorKind)>("a")(""), Err(Err::Error(("", ErrorKind::NoneOf))));
/// ```
pub fn none_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar + Copy,
  T: FindToken<<I as InputIter>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, !list.find_token(c))) {
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
    _ => Err(Err::Error(Error::from_error_kind(i, ErrorKind::NoneOf))),
  }
}

/// Recognizes the string "\r\n".
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult};
/// # use nom::character::complete::crlf;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     crlf(input)
/// }
///
/// assert_eq!(parser("\r\nc"), Ok(("c", "\r\n")));
/// assert_eq!(parser("ab\r\nc"), Err(Err::Error(Error::new("ab\r\nc", ErrorKind::CrLf))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::CrLf))));
/// ```
pub fn crlf<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>>,
  T: InputIter,
  T: Compare<&'static str>,
{
  match input.compare("\r\n") {
    //FIXME: is this the right index?
    CompareResult::Ok => Ok((input.slice(2..), input.slice(0..2))),
    _ => {
      let e: ErrorKind = ErrorKind::CrLf;
      Err(Err::Error(E::from_error_kind(input, e)))
    }
  }
}

//FIXME: there's still an incomplete
/// Recognizes a string of any char except '\r\n' or '\n'.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::not_line_ending;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     not_line_ending(input)
/// }
///
/// assert_eq!(parser("ab\r\nc"), Ok(("\r\nc", "ab")));
/// assert_eq!(parser("ab\nc"), Ok(("\nc", "ab")));
/// assert_eq!(parser("abc"), Ok(("", "abc")));
/// assert_eq!(parser(""), Ok(("", "")));
/// assert_eq!(parser("a\rb\nc"), Err(Err::Error(Error { input: "a\rb\nc", code: ErrorKind::Tag })));
/// assert_eq!(parser("a\rbc"), Err(Err::Error(Error { input: "a\rbc", code: ErrorKind::Tag })));
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
    None => Ok((input.slice(input.input_len()..), input)),
    Some(index) => {
      let mut it = input.slice(index..).iter_elements();
      let nth = it.next().unwrap().as_char();
      if nth == '\r' {
        let sliced = input.slice(index..);
        let comp = sliced.compare("\r\n");
        match comp {
          //FIXME: calculate the right index
          CompareResult::Ok => Ok((input.slice(index..), input.slice(..index))),
          _ => {
            let e: ErrorKind = ErrorKind::Tag;
            Err(Err::Error(E::from_error_kind(input, e)))
          }
        }
      } else {
        Ok((input.slice(index..), input.slice(..index)))
      }
    }
  }
}

/// Recognizes an end of line (both '\n' and '\r\n').
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::line_ending;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     line_ending(input)
/// }
///
/// assert_eq!(parser("\r\nc"), Ok(("c", "\r\n")));
/// assert_eq!(parser("ab\r\nc"), Err(Err::Error(Error::new("ab\r\nc", ErrorKind::CrLf))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::CrLf))));
/// ```
pub fn line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + InputLength,
  T: Compare<&'static str>,
{
  match input.compare("\n") {
    CompareResult::Ok => Ok((input.slice(1..), input.slice(0..1))),
    CompareResult::Incomplete => Err(Err::Error(E::from_error_kind(input, ErrorKind::CrLf))),
    CompareResult::Error => {
      match input.compare("\r\n") {
        //FIXME: is this the right index?
        CompareResult::Ok => Ok((input.slice(2..), input.slice(0..2))),
        _ => Err(Err::Error(E::from_error_kind(input, ErrorKind::CrLf))),
      }
    }
  }
}

/// Matches a newline character '\n'.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::newline;
/// fn parser(input: &str) -> IResult<&str, char> {
///     newline(input)
/// }
///
/// assert_eq!(parser("\nc"), Ok(("c", '\n')));
/// assert_eq!(parser("\r\nc"), Err(Err::Error(Error::new("\r\nc", ErrorKind::Char))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Char))));
/// ```
pub fn newline<I, Error: ParseError<I>>(input: I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar,
{
  char('\n')(input)
}

/// Matches a tab character '\t'.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::tab;
/// fn parser(input: &str) -> IResult<&str, char> {
///     tab(input)
/// }
///
/// assert_eq!(parser("\tc"), Ok(("c", '\t')));
/// assert_eq!(parser("\r\nc"), Err(Err::Error(Error::new("\r\nc", ErrorKind::Char))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Char))));
/// ```
pub fn tab<I, Error: ParseError<I>>(input: I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter,
  <I as InputIter>::Item: AsChar,
{
  char('\t')(input)
}

/// Matches one byte as a character. Note that the input type will
/// accept a `str`, but not a `&[u8]`, unlike many other nom parsers.
///
/// *Complete version*: Will return an error if there's not enough input data.
/// # Example
///
/// ```
/// # use nom::{character::complete::anychar, Err, error::{Error, ErrorKind}, IResult};
/// fn parser(input: &str) -> IResult<&str, char> {
///     anychar(input)
/// }
///
/// assert_eq!(parser("abc"), Ok(("bc",'a')));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Eof))));
/// ```
pub fn anychar<T, E: ParseError<T>>(input: T) -> IResult<T, char, E>
where
  T: InputIter + InputLength + Slice<RangeFrom<usize>>,
  <T as InputIter>::Item: AsChar,
{
  let mut it = input.iter_indices();
  match it.next() {
    None => Err(Err::Error(E::from_error_kind(input, ErrorKind::Eof))),
    Some((_, c)) => match it.next() {
      None => Ok((input.slice(input.input_len()..), c.as_char())),
      Some((idx, _)) => Ok((input.slice(idx..), c.as_char())),
    },
  }
}

/// Recognizes zero or more lowercase and uppercase ASCII alphabetic characters: a-z, A-Z
///
/// *Complete version*: Will return the whole input if no terminating token is found (a non
/// alphabetic character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::alpha0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     alpha0(input)
/// }
///
/// assert_eq!(parser("ab1c"), Ok(("1c", "ab")));
/// assert_eq!(parser("1c"), Ok(("1c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn alpha0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position_complete(|item| !item.is_alpha())
}

/// Recognizes one or more lowercase and uppercase ASCII alphabetic characters: a-z, A-Z
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found  (a non alphabetic character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::alpha1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     alpha1(input)
/// }
///
/// assert_eq!(parser("aB1c"), Ok(("1c", "aB")));
/// assert_eq!(parser("1c"), Err(Err::Error(Error::new("1c", ErrorKind::Alpha))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Alpha))));
/// ```
pub fn alpha1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1_complete(|item| !item.is_alpha(), ErrorKind::Alpha)
}

/// Recognizes zero or more ASCII numerical characters: 0-9
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::digit0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     digit0(input)
/// }
///
/// assert_eq!(parser("21c"), Ok(("c", "21")));
/// assert_eq!(parser("21"), Ok(("", "21")));
/// assert_eq!(parser("a21c"), Ok(("a21c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position_complete(|item| !item.is_dec_digit())
}

/// Recognizes one or more ASCII numerical characters: 0-9
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::digit1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     digit1(input)
/// }
///
/// assert_eq!(parser("21c"), Ok(("c", "21")));
/// assert_eq!(parser("c1"), Err(Err::Error(Error::new("c1", ErrorKind::Digit))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Digit))));
/// ```
///
/// ## Parsing an integer
/// You can use `digit1` in combination with [`map_res`] to parse an integer:
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::combinator::map_res;
/// # use nom::character::complete::digit1;
/// fn parser(input: &str) -> IResult<&str, u32> {
///   map_res(digit1, str::parse)(input)
/// }
///
/// assert_eq!(parser("416"), Ok(("", 416)));
/// assert_eq!(parser("12b"), Ok(("b", 12)));
/// assert!(parser("b").is_err());
/// ```
///
/// [`map_res`]: crate::combinator::map_res
pub fn digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1_complete(|item| !item.is_dec_digit(), ErrorKind::Digit)
}

/// Recognizes zero or more ASCII hexadecimal numerical characters: 0-9, A-F, a-f
///
/// *Complete version*: Will return the whole input if no terminating token is found (a non hexadecimal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::hex_digit0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     hex_digit0(input)
/// }
///
/// assert_eq!(parser("21cZ"), Ok(("Z", "21c")));
/// assert_eq!(parser("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn hex_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position_complete(|item| !item.is_hex_digit())
}
/// Recognizes one or more ASCII hexadecimal numerical characters: 0-9, A-F, a-f
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non hexadecimal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::hex_digit1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     hex_digit1(input)
/// }
///
/// assert_eq!(parser("21cZ"), Ok(("Z", "21c")));
/// assert_eq!(parser("H2"), Err(Err::Error(Error::new("H2", ErrorKind::HexDigit))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::HexDigit))));
/// ```
pub fn hex_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1_complete(|item| !item.is_hex_digit(), ErrorKind::HexDigit)
}

/// Recognizes zero or more octal characters: 0-7
///
/// *Complete version*: Will return the whole input if no terminating token is found (a non octal
/// digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::oct_digit0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     oct_digit0(input)
/// }
///
/// assert_eq!(parser("21cZ"), Ok(("cZ", "21")));
/// assert_eq!(parser("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn oct_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position_complete(|item| !item.is_oct_digit())
}

/// Recognizes one or more octal characters: 0-7
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non octal digit character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::oct_digit1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     oct_digit1(input)
/// }
///
/// assert_eq!(parser("21cZ"), Ok(("cZ", "21")));
/// assert_eq!(parser("H2"), Err(Err::Error(Error::new("H2", ErrorKind::OctDigit))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::OctDigit))));
/// ```
pub fn oct_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1_complete(|item| !item.is_oct_digit(), ErrorKind::OctDigit)
}

/// Recognizes zero or more ASCII numerical and alphabetic characters: 0-9, a-z, A-Z
///
/// *Complete version*: Will return the whole input if no terminating token is found (a non
/// alphanumerical character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::alphanumeric0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     alphanumeric0(input)
/// }
///
/// assert_eq!(parser("21cZ%1"), Ok(("%1", "21cZ")));
/// assert_eq!(parser("&Z21c"), Ok(("&Z21c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn alphanumeric0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position_complete(|item| !item.is_alphanum())
}

/// Recognizes one or more ASCII numerical and alphabetic characters: 0-9, a-z, A-Z
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non alphanumerical character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::alphanumeric1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     alphanumeric1(input)
/// }
///
/// assert_eq!(parser("21cZ%1"), Ok(("%1", "21cZ")));
/// assert_eq!(parser("&H2"), Err(Err::Error(Error::new("&H2", ErrorKind::AlphaNumeric))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::AlphaNumeric))));
/// ```
pub fn alphanumeric1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1_complete(|item| !item.is_alphanum(), ErrorKind::AlphaNumeric)
}

/// Recognizes zero or more spaces and tabs.
///
/// *Complete version*: Will return the whole input if no terminating token is found (a non space
/// character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::space0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     space0(input)
/// }
///
/// assert_eq!(parser(" \t21c"), Ok(("21c", " \t")));
/// assert_eq!(parser("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn space0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position_complete(|item| {
    let c = item.as_char();
    !(c == ' ' || c == '\t')
  })
}

/// Recognizes one or more spaces and tabs.
///
/// *Complete version*: Will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::space1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     space1(input)
/// }
///
/// assert_eq!(parser(" \t21c"), Ok(("21c", " \t")));
/// assert_eq!(parser("H2"), Err(Err::Error(Error::new("H2", ErrorKind::Space))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::Space))));
/// ```
pub fn space1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position1_complete(
    |item| {
      let c = item.as_char();
      !(c == ' ' || c == '\t')
    },
    ErrorKind::Space,
  )
}

/// Recognizes zero or more spaces, tabs, carriage returns and line feeds.
///
/// *Complete version*: will return the whole input if no terminating token is found (a non space
/// character).
/// # Example
///
/// ```
/// # use nom::{Err, error::ErrorKind, IResult, Needed};
/// # use nom::character::complete::multispace0;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     multispace0(input)
/// }
///
/// assert_eq!(parser(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
/// assert_eq!(parser("Z21c"), Ok(("Z21c", "")));
/// assert_eq!(parser(""), Ok(("", "")));
/// ```
pub fn multispace0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position_complete(|item| {
    let c = item.as_char();
    !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
  })
}

/// Recognizes one or more spaces, tabs, carriage returns and line feeds.
///
/// *Complete version*: will return an error if there's not enough input data,
/// or the whole input if no terminating token is found (a non space character).
/// # Example
///
/// ```
/// # use nom::{Err, error::{Error, ErrorKind}, IResult, Needed};
/// # use nom::character::complete::multispace1;
/// fn parser(input: &str) -> IResult<&str, &str> {
///     multispace1(input)
/// }
///
/// assert_eq!(parser(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
/// assert_eq!(parser("H2"), Err(Err::Error(Error::new("H2", ErrorKind::MultiSpace))));
/// assert_eq!(parser(""), Err(Err::Error(Error::new("", ErrorKind::MultiSpace))));
/// ```
pub fn multispace1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position1_complete(
    |item| {
      let c = item.as_char();
      !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
    },
    ErrorKind::MultiSpace,
  )
}

pub(crate) fn sign<T, E: ParseError<T>>(input: T) -> IResult<T, bool, E>
where
  T: Clone + InputTake,
  T: for<'a> Compare<&'a [u8]>,
{
  use crate::bytes::complete::tag;
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
                    return Err(Err::Error(E::from_error_kind(input, ErrorKind::Digit)));
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

                Ok((i.slice(i.input_len()..), value))
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
                    return Err(Err::Error(E::from_error_kind(i, ErrorKind::Digit)));
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

                Ok((i.slice(i.input_len()..), value))
            }
        )+
    }
}

uints! { u8 u16 u32 u64 u128 }

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/nom/unit/src/character/complete/tests_831.rs"));
