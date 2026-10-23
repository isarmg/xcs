// (C) Copyright 2016 Jethro G. Beekman
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//! A C expression parser and evaluator.
//!
//! This internal module provides methods for parsing and evaluating simple C expressions. In general, the
//! crate can handle most arithmetic expressions that would appear in macros or the definition of
//! constants, as well as string and character constants.
//!
//! Entry points are [`expr::expr`] for token streams and [`literal::parse`] for literals.
#![warn(rust_2018_idioms)]
#![allow(dead_code)]
#![allow(deprecated)]

mod parser;

pub mod expr;
pub mod literal;
pub mod token;

/// Parsing errors specific to C parsing
#[derive(Debug)]
pub enum ErrorKind {
    /// Expected the specified token
    ExactToken(token::Kind, &'static [u8]),
    /// Expected one of the specified tokens
    ExactTokens(token::Kind, &'static [&'static str]),
    /// Expected a token of the specified kind
    TypedToken(token::Kind),
    /// An unknown identifier was encountered
    UnknownIdentifier,
    /// An invalid literal was encountered.
    ///
    /// When encountered, this generally means a bug exists in the data that
    /// was passed in or the parsing logic.
    InvalidLiteral,
    /// A full parse was requested, but data was left over after parsing finished.
    Partial,
    /// An error occurred in an underlying slice parser.
    Parser(parser::ErrorKind),
}

impl From<parser::ErrorKind> for ErrorKind {
    fn from(k: parser::ErrorKind) -> Self {
        ErrorKind::Parser(k)
    }
}

impl From<u32> for ErrorKind {
    fn from(_: u32) -> Self {
        ErrorKind::InvalidLiteral
    }
}

/// Parsing errors specific to C parsing.
///
/// This is a superset of `(I, parser::ErrorKind)` that includes the additional errors specified by
/// [`ErrorKind`].
#[derive(Debug)]
pub struct Error<I> {
    /// The remainder of the input stream at the time of the error.
    pub input: I,
    /// The error that occurred.
    pub error: ErrorKind,
}

impl<I> From<(I, parser::ErrorKind)> for Error<I> {
    fn from(e: (I, parser::ErrorKind)) -> Self {
        Self::from((e.0, ErrorKind::from(e.1)))
    }
}

impl<I> From<(I, ErrorKind)> for Error<I> {
    fn from(e: (I, ErrorKind)) -> Self {
        Self {
            input: e.0,
            error: e.1,
        }
    }
}

impl<I> From<parser::error::Error<I>> for Error<I> {
    fn from(e: parser::error::Error<I>) -> Self {
        Self {
            input: e.input,
            error: e.code.into(),
        }
    }
}

impl<I> parser::error::ParseError<I> for Error<I> {
    fn from_error_kind(input: I, kind: parser::ErrorKind) -> Self {
        Self {
            input,
            error: kind.into(),
        }
    }

    fn append(_: I, _: parser::ErrorKind, other: Self) -> Self {
        other
    }
}

// in lieu of https://github.com/Geal/nom/issues/1010
trait ToCexprResult<I, O> {
    fn to_cexpr_result(self) -> parser::IResult<I, O, Error<I>>;
}
impl<I, O, E> ToCexprResult<I, O> for parser::IResult<I, O, E>
where
    Error<I>: From<E>,
{
    fn to_cexpr_result(self) -> parser::IResult<I, O, Error<I>> {
        match self {
            Ok(v) => Ok(v),
            Err(parser::Err::Incomplete(n)) => Err(parser::Err::Incomplete(n)),
            Err(parser::Err::Error(e)) => Err(parser::Err::Error(e.into())),
            Err(parser::Err::Failure(e)) => Err(parser::Err::Failure(e.into())),
        }
    }
}

/// If the input result indicates a succesful parse, but there is data left,
/// return an `Error::Partial` instead.
pub fn assert_full_parse<'i, I: 'i, O, E>(
    result: parser::IResult<&'i [I], O, E>,
) -> parser::IResult<&'i [I], O, Error<&'i [I]>>
where
    Error<&'i [I]>: From<E>,
{
    match result.to_cexpr_result() {
        Ok((rem, output)) => {
            if rem.is_empty() {
                Ok((rem, output))
            } else {
                Err(parser::Err::Error((rem, ErrorKind::Partial).into()))
            }
        }
        Err(parser::Err::Incomplete(n)) => Err(parser::Err::Incomplete(n)),
        Err(parser::Err::Failure(e)) => Err(parser::Err::Failure(e)),
        Err(parser::Err::Error(e)) => Err(parser::Err::Error(e)),
    }
}
