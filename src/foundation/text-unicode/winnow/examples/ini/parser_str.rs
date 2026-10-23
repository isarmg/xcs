use std::collections::HashMap;

use winnow::prelude::*;
use winnow::Result;
use winnow::{
    ascii::{alphanumeric1 as alphanumeric, space0 as space},
    combinator::opt,
    combinator::repeat,
    combinator::{delimited, terminated},
    token::{take_till, take_while},
};

pub(crate) type Stream<'i> = &'i str;

pub(crate) fn categories<'s>(
    input: &mut Stream<'s>,
) -> Result<HashMap<&'s str, HashMap<&'s str, &'s str>>> {
    repeat(0.., category_and_keys).parse_next(input)
}

fn category_and_keys<'s>(i: &mut Stream<'s>) -> Result<(&'s str, HashMap<&'s str, &'s str>)> {
    (category, keys_and_values).parse_next(i)
}

fn category<'s>(i: &mut Stream<'s>) -> Result<&'s str> {
    terminated(
        delimited('[', take_while(0.., |c| c != ']'), ']'),
        opt(take_while(1.., [' ', '\r', '\n'])),
    )
    .parse_next(i)
}

fn keys_and_values<'s>(input: &mut Stream<'s>) -> Result<HashMap<&'s str, &'s str>> {
    repeat(0.., key_value).parse_next(input)
}

fn key_value<'s>(i: &mut Stream<'s>) -> Result<(&'s str, &'s str)> {
    let key = alphanumeric.parse_next(i)?;
    let _ = (opt(space), "=", opt(space)).parse_next(i)?;
    let val = take_till(0.., is_line_ending_or_comment).parse_next(i)?;
    let _ = opt(space).parse_next(i)?;
    let _ = opt((";", till_line_ending)).parse_next(i)?;
    let _ = opt(space_or_line_ending).parse_next(i)?;

    Ok((key, val))
}

fn is_line_ending_or_comment(chr: char) -> bool {
    chr == ';' || chr == '\n'
}

fn till_line_ending<'s>(i: &mut Stream<'s>) -> Result<&'s str> {
    take_while(0.., |c| c != '\r' && c != '\n').parse_next(i)
}

fn space_or_line_ending<'s>(i: &mut Stream<'s>) -> Result<&'s str> {
    take_while(1.., [' ', '\r', '\n']).parse_next(i)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_category_test_60.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_key_value_test_80.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_key_value_with_space_test_97.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_key_value_with_comment_test_114.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_multiple_keys_and_values_test_131.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_category_then_multiple_keys_and_values_test_154.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser_str/parse_multiple_categories_test_179.rs"));
