use std::collections::HashMap;
use std::str;

use winnow::prelude::*;
use winnow::{
    ascii::float,
    combinator::alt,
    combinator::cut_err,
    combinator::{delimited, preceded, separated_pair, terminated},
    combinator::{repeat, separated},
    error::{AddContext, ParserError, StrContext},
    stream::Partial,
    token::{any, none_of, rest, take, take_while},
};

use crate::json::JsonValue;

pub(crate) type Stream<'i> = Partial<&'i str>;

/// The root element of a JSON parser is any value
///
/// A parser has the following signature:
/// `&mut Stream -> ModalResult<Output, ContextError>`, with `ModalResult` defined as:
/// `type ModalResult<O, E = ErrorKind> = Result<O, ErrMode<E>>;`
///
/// most of the times you can ignore the error type and use the default (but this
/// examples shows custom error types later on!)
///
/// Here we use `&str` as input type, but parsers can be generic over
/// the input type, work directly with `&[u8]`, or any other type that
/// implements the required traits.
pub(crate) fn json<'i, E: ParserError<Stream<'i>> + AddContext<Stream<'i>, StrContext>>(
    input: &mut Stream<'i>,
) -> ModalResult<JsonValue, E> {
    delimited(ws, json_value, ws_or_eof).parse_next(input)
}

/// `alt` is a combinator that tries multiple parsers one by one, until
/// one of them succeeds
fn json_value<'i, E: ParserError<Stream<'i>> + AddContext<Stream<'i>, StrContext>>(
    input: &mut Stream<'i>,
) -> ModalResult<JsonValue, E> {
    // `alt` combines the each value parser. It returns the result of the first
    // successful parser, or an error
    alt((
        null.value(JsonValue::Null),
        boolean.map(JsonValue::Boolean),
        string.map(JsonValue::Str),
        float.map(JsonValue::Num),
        array.map(JsonValue::Array),
        object.map(JsonValue::Object),
    ))
    .parse_next(input)
}

/// `literal(string)` generates a parser that takes the argument string.
///
/// This also shows returning a sub-slice of the original input
fn null<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<&'i str, E> {
    // This is a parser that returns `"null"` if it sees the string "null", and
    // an error otherwise
    "null".parse_next(input)
}

/// We can combine `tag` with other functions, like `value` which returns a given constant value on
/// success.
fn boolean<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<bool, E> {
    // This is a parser that returns `true` if it sees the string "true", and
    // an error otherwise
    let parse_true = "true".value(true);

    // This is a parser that returns `false` if it sees the string "false", and
    // an error otherwise
    let parse_false = "false".value(false);

    alt((parse_true, parse_false)).parse_next(input)
}

/// This parser gathers all `char`s up into a `String`with a parse to take the double quote
/// character, before the string (using `preceded`) and after the string (using `terminated`).
fn string<'i, E: ParserError<Stream<'i>> + AddContext<Stream<'i>, StrContext>>(
    input: &mut Stream<'i>,
) -> ModalResult<String, E> {
    preceded(
        '\"',
        // `cut_err` transforms an `ErrMode::Backtrack(e)` to `ErrMode::Cut(e)`, signaling to
        // combinators like  `alt` that they should not try other parsers. We were in the
        // right branch (since we found the `"` character) but encountered an error when
        // parsing the string
        cut_err(terminated(
            repeat(0.., character).fold(String::new, |mut string, c| {
                string.push(c);
                string
            }),
            '\"',
        )),
    )
    // `context` lets you add a static string to errors to provide more information in the
    // error chain (to indicate which parser had an error)
    .context(StrContext::Expected("string".into()))
    .parse_next(input)
}

/// You can mix the above declarative parsing with an imperative style to handle more unique cases,
/// like escaping
fn character<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<char, E> {
    let c = none_of('\"').parse_next(input)?;
    if c == '\\' {
        alt((
            any.verify_map(|c| {
                Some(match c {
                    '"' | '\\' | '/' => c,
                    'b' => '\x08',
                    'f' => '\x0C',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => return None,
                })
            }),
            preceded('u', unicode_escape),
        ))
        .parse_next(input)
    } else {
        Ok(c)
    }
}

fn unicode_escape<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<char, E> {
    alt((
        // Not a surrogate
        u16_hex
            .verify(|cp| !(0xD800..0xE000).contains(cp))
            .map(|cp| cp as u32),
        // See https://en.wikipedia.org/wiki/UTF-16#Code_points_from_U+010000_to_U+10FFFF for details
        separated_pair(u16_hex, "\\u", u16_hex)
            .verify(|(high, low)| (0xD800..0xDC00).contains(high) && (0xDC00..0xE000).contains(low))
            .map(|(high, low)| {
                let high_ten = (high as u32) - 0xD800;
                let low_ten = (low as u32) - 0xDC00;
                (high_ten << 10) + low_ten + 0x10000
            }),
    ))
    .verify_map(
        // Could be probably replaced with .unwrap() or _unchecked due to the verify checks
        std::char::from_u32,
    )
    .parse_next(input)
}

fn u16_hex<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<u16, E> {
    take(4usize)
        .verify_map(|s| u16::from_str_radix(s, 16).ok())
        .parse_next(input)
}

/// Some combinators, like `separated` or `repeat`, will call a parser repeatedly,
/// accumulating results in a `Vec`, until it encounters an error.
/// If you want more control on the parser application, check out the `iterator`
/// combinator (cf `examples/iterator.rs`)
fn array<'i, E: ParserError<Stream<'i>> + AddContext<Stream<'i>, StrContext>>(
    input: &mut Stream<'i>,
) -> ModalResult<Vec<JsonValue>, E> {
    preceded(
        ('[', ws),
        cut_err(terminated(
            separated(0.., json_value, (ws, ',', ws)),
            (ws, ']'),
        )),
    )
    .context(StrContext::Expected("array".into()))
    .parse_next(input)
}

fn object<'i, E: ParserError<Stream<'i>> + AddContext<Stream<'i>, StrContext>>(
    input: &mut Stream<'i>,
) -> ModalResult<HashMap<String, JsonValue>, E> {
    preceded(
        ('{', ws),
        cut_err(terminated(
            separated(0.., key_value, (ws, ',', ws)),
            (ws, '}'),
        )),
    )
    .context(StrContext::Expected("object".into()))
    .parse_next(input)
}

fn key_value<'i, E: ParserError<Stream<'i>> + AddContext<Stream<'i>, StrContext>>(
    input: &mut Stream<'i>,
) -> ModalResult<(String, JsonValue), E> {
    separated_pair(string, cut_err((ws, ':', ws)), json_value).parse_next(input)
}

/// Parser combinators are constructed from the bottom up:
/// first we write parsers for the smallest elements (here a space character),
/// then we'll combine them in larger parsers
fn ws<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<&'i str, E> {
    // Combinators like `take_while` return a function. That function is the
    // parser,to which we can pass the input
    take_while(0.., WS).parse_next(input)
}

fn ws_or_eof<'i, E: ParserError<Stream<'i>>>(input: &mut Stream<'i>) -> ModalResult<&'i str, E> {
    rest.verify(|s: &str| s.chars().all(|c| WS.contains(&c)))
        .parse_next(input)
}

const WS: &[char] = &[' ', '\t', '\r', '\n'];

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/winnow/unit/examples/json/parser_partial/test_211.rs"));
