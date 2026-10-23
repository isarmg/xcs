use std::collections::HashMap;
use std::str;

use winnow::prelude::*;
use winnow::Result;
use winnow::{
    ascii::{alphanumeric1 as alphanumeric, multispace0 as multispace, space0 as space},
    combinator::opt,
    combinator::repeat,
    combinator::{delimited, separated_pair, terminated},
    token::take_while,
};

pub(crate) type Stream<'i> = &'i [u8];

pub(crate) fn categories<'s>(
    i: &mut Stream<'s>,
) -> Result<HashMap<&'s str, HashMap<&'s str, &'s str>>> {
    repeat(
        0..,
        separated_pair(
            category,
            opt(multispace),
            repeat(0.., terminated(key_value, opt(multispace))),
        ),
    )
    .parse_next(i)
}

fn category<'s>(i: &mut Stream<'s>) -> Result<&'s str> {
    delimited('[', take_while(0.., |c| c != b']'), ']')
        .try_map(str::from_utf8)
        .parse_next(i)
}

pub(crate) fn key_value<'s>(i: &mut Stream<'s>) -> Result<(&'s str, &'s str)> {
    let key = alphanumeric.try_map(str::from_utf8).parse_next(i)?;
    let _ = (opt(space), '=', opt(space)).parse_next(i)?;
    let val = take_while(0.., |c| c != b'\n' && c != b';')
        .try_map(str::from_utf8)
        .parse_next(i)?;
    let _ = opt((';', take_while(0.., |c| c != b'\n'))).parse_next(i)?;
    Ok((key, val))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser/parse_category_test_46.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser/parse_key_value_test_66.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser/parse_key_value_with_space_test_83.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser/parse_key_value_with_comment_test_100.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/winnow/functions/examples/ini/parser/parse_multiple_categories_test_117.rs"));
