use winnow::combinator::alt;
use winnow::combinator::fail;
use winnow::combinator::peek;
use winnow::token::any;

use crate::parser::array::array;
use crate::parser::datetime::date_time;
use crate::parser::inline_table::inline_table;
use crate::parser::numbers::{float, integer};
use crate::parser::prelude::*;
use crate::parser::strings::string;
use crate::repr::{Formatted, Repr};
use crate::RawString;
use crate::Value;

// val = string / boolean / array / inline-table / date-time / float / integer
pub(crate) fn value(input: &mut Input<'_>) -> ModalResult<Value> {
    dispatch! {peek(any);
            crate::parser::strings::QUOTATION_MARK |
            crate::parser::strings::APOSTROPHE => string.map(|s| {
                Value::String(Formatted::new(
                    s.into_owned()
                ))
            }),
            crate::parser::array::ARRAY_OPEN => check_recursion(array).map(Value::Array),
            crate::parser::inline_table::INLINE_TABLE_OPEN => check_recursion(inline_table).map(Value::InlineTable),
            // Date/number starts
            b'+' | b'-' | b'0'..=b'9' => {
                // Uncommon enough not to be worth optimizing at this time
                alt((
                    date_time
                        .map(Value::from),
                    float
                        .map(Value::from),
                    integer
                        .map(Value::from),
                ))
            },
            // Report as if they were numbers because its most likely a typo
            b'_' => {
                    integer
                        .map(Value::from)
                .context(StrContext::Expected(StrContextValue::Description("leading digit")))
            },
            // Report as if they were numbers because its most likely a typo
            b'.' =>  {
                    float
                        .map(Value::from)
                .context(StrContext::Expected(StrContextValue::Description("leading digit")))
            },
            b't' => {
                crate::parser::numbers::true_.map(Value::from)
                    .context(StrContext::Label("string"))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('"')))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('\'')))
            },
            b'f' => {
                crate::parser::numbers::false_.map(Value::from)
                    .context(StrContext::Label("string"))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('"')))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('\'')))
            },
            b'i' => {
                crate::parser::numbers::inf.map(Value::from)
                    .context(StrContext::Label("string"))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('"')))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('\'')))
            },
            b'n' => {
                crate::parser::numbers::nan.map(Value::from)
                    .context(StrContext::Label("string"))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('"')))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('\'')))
            },
            _ => {
                fail
                    .context(StrContext::Label("string"))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('"')))
                    .context(StrContext::Expected(StrContextValue::CharLiteral('\'')))
            },
    }
    .with_span()
    .map(|(value, span)| apply_raw(value, span))
    .parse_next(input)
}

fn apply_raw(mut val: Value, span: std::ops::Range<usize>) -> Value {
    match val {
        Value::String(ref mut f) => {
            let raw = RawString::with_span(span);
            f.set_repr_unchecked(Repr::new_unchecked(raw));
        }
        Value::Integer(ref mut f) => {
            let raw = RawString::with_span(span);
            f.set_repr_unchecked(Repr::new_unchecked(raw));
        }
        Value::Float(ref mut f) => {
            let raw = RawString::with_span(span);
            f.set_repr_unchecked(Repr::new_unchecked(raw));
        }
        Value::Boolean(ref mut f) => {
            let raw = RawString::with_span(span);
            f.set_repr_unchecked(Repr::new_unchecked(raw));
        }
        Value::Datetime(ref mut f) => {
            let raw = RawString::with_span(span);
            f.set_repr_unchecked(Repr::new_unchecked(raw));
        }
        Value::Array(ref mut arr) => {
            arr.span = Some(span);
        }
        Value::InlineTable(ref mut table) => {
            table.span = Some(span);
        }
    };
    val.decorate("", "");
    val
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/toml_edit/unit/src/parser/value/test_120.rs"));
