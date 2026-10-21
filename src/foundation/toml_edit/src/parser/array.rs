use winnow::combinator::cut_err;
use winnow::combinator::delimited;
use winnow::combinator::opt;
use winnow::combinator::peek;
use winnow::combinator::separated;

use crate::parser::trivia::ws_comment_newline;
use crate::parser::value::value;
use crate::{Array, Item, RawString};

use crate::parser::prelude::*;

// ;; Array

// array = array-open array-values array-close
pub(crate) fn array<'i>(input: &mut Input<'i>) -> ModalResult<Array> {
    (move |input: &mut Input<'i>| {
        delimited(
            ARRAY_OPEN,
            cut_err(array_values),
            cut_err(ARRAY_CLOSE)
                .context(StrContext::Label("array"))
                .context(StrContext::Expected(StrContextValue::CharLiteral(']'))),
        )
        .parse_next(input)
    })
    .parse_next(input)
}

// note: we're omitting ws and newlines here, because
// they should be part of the formatted values
// array-open  = %x5B ws-newline  ; [
pub(crate) const ARRAY_OPEN: u8 = b'[';
// array-close = ws-newline %x5D  ; ]
const ARRAY_CLOSE: u8 = b']';
// array-sep = ws %x2C ws  ; , Comma
const ARRAY_SEP: u8 = b',';

// array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
// array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
fn array_values(input: &mut Input<'_>) -> ModalResult<Array> {
    if peek(opt(ARRAY_CLOSE)).parse_next(input)?.is_some() {
        // Optimize for empty arrays, avoiding `value` from being expected to fail
        return Ok(Array::new());
    }

    let array = separated(0.., array_value, ARRAY_SEP).parse_next(input)?;
    let mut array = Array::with_vec(array);
    if !array.is_empty() {
        let comma = opt(ARRAY_SEP).parse_next(input)?.is_some();
        array.set_trailing_comma(comma);
    }
    let trailing = ws_comment_newline.span().parse_next(input)?;
    array.set_trailing(RawString::with_span(trailing));

    Ok(array)
}

fn array_value(input: &mut Input<'_>) -> ModalResult<Item> {
    let prefix = ws_comment_newline.span().parse_next(input)?;
    let value = value.parse_next(input)?;
    let suffix = ws_comment_newline.span().parse_next(input)?;
    let value = value.decorated(RawString::with_span(prefix), RawString::with_span(suffix));
    let value = Item::Value(value);
    Ok(value)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/toml_edit/unit/src/parser/array/test_68.rs"));
