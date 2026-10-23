use std::ops::RangeInclusive;

use winnow::combinator::alt;
use winnow::combinator::cut_err;
use winnow::combinator::opt;
use winnow::combinator::peek;
use winnow::combinator::preceded;
use winnow::combinator::repeat;
use winnow::token::one_of;
use winnow::token::rest;
use winnow::token::take;

use crate::editor::parser::prelude::*;
use crate::editor::parser::trivia::from_utf8_unchecked;

// ;; Boolean

// boolean = true / false
#[allow(dead_code)] // directly define in `fn value`
pub(crate) fn boolean(input: &mut Input<'_>) -> ModalResult<bool> {
    alt((true_, false_)).parse_next(input)
}

pub(crate) fn true_(input: &mut Input<'_>) -> ModalResult<bool> {
    (peek(TRUE[0]), cut_err(TRUE)).value(true).parse_next(input)
}
const TRUE: &[u8] = b"true";

pub(crate) fn false_(input: &mut Input<'_>) -> ModalResult<bool> {
    (peek(FALSE[0]), cut_err(FALSE))
        .value(false)
        .parse_next(input)
}
const FALSE: &[u8] = b"false";

// ;; Integer

// integer = dec-int / hex-int / oct-int / bin-int
pub(crate) fn integer(input: &mut Input<'_>) -> ModalResult<i64> {
    dispatch! {peek(opt::<_, &[u8], _, _>(take(2usize)));
        Some(b"0x") => cut_err(hex_int.try_map(|s| i64::from_str_radix(&s.replace('_', ""), 16))),
        Some(b"0o") => cut_err(oct_int.try_map(|s| i64::from_str_radix(&s.replace('_', ""), 8))),
        Some(b"0b") => cut_err(bin_int.try_map(|s| i64::from_str_radix(&s.replace('_', ""), 2))),
        _ => dec_int.and_then(cut_err(rest
            .try_map(|s: &str| s.replace('_', "").parse())))
    }
    .parse_next(input)
}

// dec-int = [ minus / plus ] unsigned-dec-int
// unsigned-dec-int = DIGIT / digit1-9 1*( DIGIT / underscore DIGIT )
pub(crate) fn dec_int<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    (
        opt(one_of((b'+', b'-'))),
        alt((
            (
                one_of(DIGIT1_9),
                repeat(
                    0..,
                    alt((
                        digit.void(),
                        (
                            one_of(b'_'),
                            cut_err(digit).context(StrContext::Expected(
                                StrContextValue::Description("digit"),
                            )),
                        )
                            .void(),
                    )),
                )
                .map(|()| ()),
            )
                .void(),
            digit.void(),
        )),
    )
        .take()
        .map(|b: &[u8]| unsafe { from_utf8_unchecked(b, "`digit` and `_` filter out non-ASCII") })
        .context(StrContext::Label("integer"))
        .parse_next(input)
}
const DIGIT1_9: RangeInclusive<u8> = b'1'..=b'9';

// hex-prefix = %x30.78               ; 0x
// hex-int = hex-prefix HEXDIG *( HEXDIG / underscore HEXDIG )
pub(crate) fn hex_int<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    preceded(
        HEX_PREFIX,
        cut_err((
            hexdig,
            repeat(
                0..,
                alt((
                    hexdig.void(),
                    (
                        one_of(b'_'),
                        cut_err(hexdig)
                            .context(StrContext::Expected(StrContextValue::Description("digit"))),
                    )
                        .void(),
                )),
            )
            .map(|()| ()),
        ))
        .take(),
    )
    .map(|b| unsafe { from_utf8_unchecked(b, "`hexdig` and `_` filter out non-ASCII") })
    .context(StrContext::Label("hexadecimal integer"))
    .parse_next(input)
}
const HEX_PREFIX: &[u8] = b"0x";

// oct-prefix = %x30.6F               ; 0o
// oct-int = oct-prefix digit0-7 *( digit0-7 / underscore digit0-7 )
pub(crate) fn oct_int<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    preceded(
        OCT_PREFIX,
        cut_err((
            one_of(DIGIT0_7),
            repeat(
                0..,
                alt((
                    one_of(DIGIT0_7).void(),
                    (
                        one_of(b'_'),
                        cut_err(one_of(DIGIT0_7))
                            .context(StrContext::Expected(StrContextValue::Description("digit"))),
                    )
                        .void(),
                )),
            )
            .map(|()| ()),
        ))
        .take(),
    )
    .map(|b| unsafe { from_utf8_unchecked(b, "`DIGIT0_7` and `_` filter out non-ASCII") })
    .context(StrContext::Label("octal integer"))
    .parse_next(input)
}
const OCT_PREFIX: &[u8] = b"0o";
const DIGIT0_7: RangeInclusive<u8> = b'0'..=b'7';

// bin-prefix = %x30.62               ; 0b
// bin-int = bin-prefix digit0-1 *( digit0-1 / underscore digit0-1 )
pub(crate) fn bin_int<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    preceded(
        BIN_PREFIX,
        cut_err((
            one_of(DIGIT0_1),
            repeat(
                0..,
                alt((
                    one_of(DIGIT0_1).void(),
                    (
                        one_of(b'_'),
                        cut_err(one_of(DIGIT0_1))
                            .context(StrContext::Expected(StrContextValue::Description("digit"))),
                    )
                        .void(),
                )),
            )
            .map(|()| ()),
        ))
        .take(),
    )
    .map(|b| unsafe { from_utf8_unchecked(b, "`DIGIT0_1` and `_` filter out non-ASCII") })
    .context(StrContext::Label("binary integer"))
    .parse_next(input)
}
const BIN_PREFIX: &[u8] = b"0b";
const DIGIT0_1: RangeInclusive<u8> = b'0'..=b'1';

// ;; Float

// float = float-int-part ( exp / frac [ exp ] )
// float =/ special-float
// float-int-part = dec-int
pub(crate) fn float(input: &mut Input<'_>) -> ModalResult<f64> {
    alt((
        float_.and_then(cut_err(
            rest.try_map(|s: &str| s.replace('_', "").parse())
                .verify(|f: &f64| *f != f64::INFINITY),
        )),
        special_float,
    ))
    .context(StrContext::Label("floating-point number"))
    .parse_next(input)
}

fn float_<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    (
        dec_int,
        alt((exp.void(), (frac.void(), opt(exp.void())).void())),
    )
        .take()
        .map(|b: &[u8]| unsafe {
            from_utf8_unchecked(
                b,
                "`dec_int`, `one_of`, `exp`, and `frac` filter out non-ASCII",
            )
        })
        .parse_next(input)
}

// frac = decimal-point zero-prefixable-int
// decimal-point = %x2E               ; .
fn frac<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    (
        b'.',
        cut_err(zero_prefixable_int)
            .context(StrContext::Expected(StrContextValue::Description("digit"))),
    )
        .take()
        .map(|b: &[u8]| unsafe {
            from_utf8_unchecked(
                b,
                "`.` and `parse_zero_prefixable_int` filter out non-ASCII",
            )
        })
        .parse_next(input)
}

// zero-prefixable-int = DIGIT *( DIGIT / underscore DIGIT )
fn zero_prefixable_int<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    (
        digit,
        repeat(
            0..,
            alt((
                digit.void(),
                (
                    one_of(b'_'),
                    cut_err(digit)
                        .context(StrContext::Expected(StrContextValue::Description("digit"))),
                )
                    .void(),
            )),
        )
        .map(|()| ()),
    )
        .take()
        .map(|b: &[u8]| unsafe { from_utf8_unchecked(b, "`digit` and `_` filter out non-ASCII") })
        .parse_next(input)
}

// exp = "e" float-exp-part
// float-exp-part = [ minus / plus ] zero-prefixable-int
fn exp<'i>(input: &mut Input<'i>) -> ModalResult<&'i str> {
    (
        one_of((b'e', b'E')),
        opt(one_of([b'+', b'-'])),
        cut_err(zero_prefixable_int),
    )
        .take()
        .map(|b: &[u8]| unsafe {
            from_utf8_unchecked(
                b,
                "`one_of` and `parse_zero_prefixable_int` filter out non-ASCII",
            )
        })
        .parse_next(input)
}

// special-float = [ minus / plus ] ( inf / nan )
fn special_float(input: &mut Input<'_>) -> ModalResult<f64> {
    (opt(one_of((b'+', b'-'))), alt((inf, nan)))
        .map(|(s, f)| match s {
            Some(b'+') | None => f,
            Some(b'-') => -f,
            _ => unreachable!("one_of should prevent this"),
        })
        .parse_next(input)
}
// inf = %x69.6e.66  ; inf
pub(crate) fn inf(input: &mut Input<'_>) -> ModalResult<f64> {
    INF.value(f64::INFINITY).parse_next(input)
}
const INF: &[u8] = b"inf";
// nan = %x6e.61.6e  ; nan
pub(crate) fn nan(input: &mut Input<'_>) -> ModalResult<f64> {
    NAN.value(f64::NAN.copysign(1.0)).parse_next(input)
}
const NAN: &[u8] = b"nan";

// DIGIT = %x30-39 ; 0-9
fn digit(input: &mut Input<'_>) -> ModalResult<u8> {
    one_of(DIGIT).parse_next(input)
}
const DIGIT: RangeInclusive<u8> = b'0'..=b'9';

// HEXDIG = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
fn hexdig(input: &mut Input<'_>) -> ModalResult<u8> {
    one_of(HEXDIG).parse_next(input)
}
pub(crate) const HEXDIG: (RangeInclusive<u8>, RangeInclusive<u8>, RangeInclusive<u8>) =
    (DIGIT, b'A'..=b'F', b'a'..=b'f');

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/toml/editor/unit/src/parser/numbers/test_298.rs"
));
