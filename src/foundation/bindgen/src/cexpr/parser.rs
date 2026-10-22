// Derived from the behavior of nom 7.1.3 (MIT), copyright Geoffroy Couprie.
// License: compliance/licenses/upstream/nom-7.1.3/LICENSE.
//! Slice-only parser operations needed by the C expression grammar.
//!
//! Unlike the former general parser dependency this has no string, bit-stream,
//! numeric, permutation, iterator, or lexical-conversion machinery. Alternatives
//! backtrack only on Error; Incomplete and Failure propagate unchanged.

use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Needed {
    Unknown,
    Size(NonZeroUsize),
}
impl Needed {
    pub fn new(size: usize) -> Self {
        NonZeroUsize::new(size).map_or(Self::Unknown, Self::Size)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Err<E> {
    Incomplete(Needed),
    Error(E),
    Failure(E),
}
pub type IResult<I, O, E = error::Error<I>> = Result<(I, O), Err<E>>;

pub mod error {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ErrorKind {
        Alt,
        Tag,
        IsNot,
        Char,
        OneOf,
        Complete,
        MapOpt,
        Many0,
        Many1,
        ManyMN,
        SeparatedList,
        Fix,
    }
    #[derive(Clone, Debug, PartialEq)]
    pub struct Error<I> {
        pub input: I,
        pub code: ErrorKind,
    }
    impl<I> Error<I> {
        pub fn new(input: I, code: ErrorKind) -> Self {
            Self { input, code }
        }
    }
    pub trait ParseError<I>: Sized {
        fn from_error_kind(input: I, kind: ErrorKind) -> Self;
        fn append(input: I, kind: ErrorKind, other: Self) -> Self;
    }
    impl<I> ParseError<I> for Error<I> {
        fn from_error_kind(input: I, kind: ErrorKind) -> Self {
            Self::new(input, kind)
        }
        fn append(_: I, _: ErrorKind, other: Self) -> Self {
            other
        }
    }
}
pub use error::ErrorKind;
use error::ParseError;

pub trait InputLength {
    fn input_len(&self) -> usize;
}
impl<T> InputLength for &[T] {
    fn input_len(&self) -> usize {
        self.len()
    }
}

pub mod branch {
    use super::*;
    pub trait Alt<I, O, E> {
        fn choice(&mut self, input: I) -> IResult<I, O, E>;
    }
    pub fn alt<I, O, E, A: Alt<I, O, E>>(mut alternatives: A) -> impl FnMut(I) -> IResult<I, O, E> {
        move |input| alternatives.choice(input)
    }
    macro_rules! alternatives {
        ($($name:ident:$field:tt),+) => {
            impl<I: Clone, O, E: ParseError<I>, $($name),+> Alt<I,O,E> for ($($name,)+)
            where $($name: FnMut(I) -> IResult<I,O,E>,)+ {
                fn choice(&mut self, input: I) -> IResult<I,O,E> {
                    let mut last = None;
                    $(match self.$field(input.clone()) {
                        Err(Err::Error(e)) => { let _ = last.take(); last = Some(e); },
                        result => return result,
                    })+
                    Err(Err::Error(E::append(input, ErrorKind::Alt, last.unwrap())))
                }
            }
        }
    }
    alternatives!(A:0,B:1);
    alternatives!(A:0,B:1,C:2);
    alternatives!(A:0,B:1,C:2,D:3);
    alternatives!(A:0,B:1,C:2,D:3,F:4);
    alternatives!(A:0,B:1,C:2,D:3,F:4,G:5);
    alternatives!(A:0,B:1,C:2,D:3,F:4,G:5,H:6);
}
pub mod combinator {
    use super::*;
    pub fn complete<I: Clone, O, E: ParseError<I>, F: FnMut(I) -> IResult<I, O, E>>(
        mut f: F,
    ) -> impl FnMut(I) -> IResult<I, O, E> {
        move |input| match f(input.clone()) {
            Err(Err::Incomplete(_)) => {
                Err(Err::Error(E::from_error_kind(input, ErrorKind::Complete)))
            }
            result => result,
        }
    }
    pub fn map<I, O, P, E, F: FnMut(I) -> IResult<I, O, E>, G: FnMut(O) -> P>(
        mut f: F,
        mut g: G,
    ) -> impl FnMut(I) -> IResult<I, P, E> {
        move |input| f(input).map(|(rest, out)| (rest, g(out)))
    }
    pub fn map_opt<
        I: Clone,
        O,
        P,
        E: ParseError<I>,
        F: FnMut(I) -> IResult<I, O, E>,
        G: FnMut(O) -> Option<P>,
    >(
        mut f: F,
        mut g: G,
    ) -> impl FnMut(I) -> IResult<I, P, E> {
        move |input| {
            let (rest, out) = f(input.clone())?;
            g(out)
                .map(|out| (rest, out))
                .ok_or_else(|| Err::Error(E::from_error_kind(input, ErrorKind::MapOpt)))
        }
    }
    pub fn opt<I: Clone, O, E, F: FnMut(I) -> IResult<I, O, E>>(
        mut f: F,
    ) -> impl FnMut(I) -> IResult<I, Option<O>, E> {
        move |input| match f(input.clone()) {
            Ok((rest, out)) => Ok((rest, Some(out))),
            Err(Err::Error(_)) => Ok((input, None)),
            Err(e) => Err(e),
        }
    }
    pub fn recognize<'a, T: 'a, O, E, F: FnMut(&'a [T]) -> IResult<&'a [T], O, E>>(
        mut f: F,
    ) -> impl FnMut(&'a [T]) -> IResult<&'a [T], &'a [T], E> {
        move |input| f(input).map(|(rest, _)| (rest, &input[..input.len() - rest.len()]))
    }
}
pub mod sequence {
    use super::*;
    pub fn pair<I, O, P, E, F: FnMut(I) -> IResult<I, O, E>, G: FnMut(I) -> IResult<I, P, E>>(
        mut f: F,
        mut g: G,
    ) -> impl FnMut(I) -> IResult<I, (O, P), E> {
        move |input| {
            let (input, a) = f(input)?;
            let (input, b) = g(input)?;
            Ok((input, (a, b)))
        }
    }
    pub fn preceded<
        I,
        O,
        P,
        E,
        F: FnMut(I) -> IResult<I, O, E>,
        G: FnMut(I) -> IResult<I, P, E>,
    >(
        mut f: F,
        mut g: G,
    ) -> impl FnMut(I) -> IResult<I, P, E> {
        move |input| {
            let (input, _) = f(input)?;
            g(input)
        }
    }
    pub fn terminated<
        I,
        O,
        P,
        E,
        F: FnMut(I) -> IResult<I, O, E>,
        G: FnMut(I) -> IResult<I, P, E>,
    >(
        mut f: F,
        mut g: G,
    ) -> impl FnMut(I) -> IResult<I, O, E> {
        move |input| {
            let (input, a) = f(input)?;
            let (input, _) = g(input)?;
            Ok((input, a))
        }
    }
    pub fn delimited<
        I,
        O,
        P,
        Q,
        E,
        F: FnMut(I) -> IResult<I, O, E>,
        G: FnMut(I) -> IResult<I, P, E>,
        H: FnMut(I) -> IResult<I, Q, E>,
    >(
        f: F,
        g: G,
        h: H,
    ) -> impl FnMut(I) -> IResult<I, P, E> {
        preceded(f, terminated(g, h))
    }
    pub trait Tuple<I, O, E> {
        fn parse(&mut self, input: I) -> IResult<I, O, E>;
    }
    pub fn tuple<I, O, E, F: Tuple<I, O, E>>(mut f: F) -> impl FnMut(I) -> IResult<I, O, E> {
        move |input| f.parse(input)
    }
    macro_rules! tuples {
        ($($f:ident:$o:ident:$field:tt),+) => {
            impl<I,E,$($o,$f),+> Tuple<I,($($o,)+),E> for ($($f,)+)
            where $($f:FnMut(I)->IResult<I,$o,E>,)+ {
                fn parse(&mut self,input:I)->IResult<I,($($o,)+),E> {
                    $(let (input,$o)=self.$field(input)?;)+
                    Ok((input,($($o,)+)))
                }
            }
        }
    }
    #[allow(non_snake_case)]
    mod tuple_impls {
        use super::*;
        tuples!(A:O:0,B:P:1,C:Q:2);
        tuples!(A:O:0,B:P:1,C:Q:2,D:R:3);
    }
}
pub mod multi {
    use super::*;
    pub fn fold_many0<
        I: Clone + InputLength,
        O,
        A,
        E: ParseError<I>,
        F: FnMut(I) -> IResult<I, O, E>,
        G: FnMut() -> A,
        H: FnMut(A, O) -> A,
    >(
        mut f: F,
        mut init: G,
        mut fold: H,
    ) -> impl FnMut(I) -> IResult<I, A, E> {
        move |mut input| {
            let mut acc = init();
            loop {
                match f(input.clone()) {
                    Ok((rest, out)) => {
                        if rest.input_len() == input.input_len() {
                            return Err(Err::Error(E::from_error_kind(input, ErrorKind::Many0)));
                        }
                        input = rest;
                        acc = fold(acc, out);
                    }
                    Err(Err::Error(_)) => return Ok((input, acc)),
                    Err(e) => return Err(e),
                }
            }
        }
    }
    pub fn many0<I: Clone + InputLength, O, E: ParseError<I>, F: FnMut(I) -> IResult<I, O, E>>(
        f: F,
    ) -> impl FnMut(I) -> IResult<I, Vec<O>, E> {
        fold_many0(f, Vec::new, |mut v, x| {
            v.push(x);
            v
        })
    }
    pub fn many1<I: Clone + InputLength, O, E: ParseError<I>, F: FnMut(I) -> IResult<I, O, E>>(
        mut f: F,
    ) -> impl FnMut(I) -> IResult<I, Vec<O>, E> {
        move |input| {
            let (mut input, first) = match f(input.clone()) {
                Err(Err::Error(e)) => {
                    return Err(Err::Error(E::append(input, ErrorKind::Many1, e)))
                }
                result => result?,
            };
            let mut values = vec![first];
            loop {
                match f(input.clone()) {
                    Ok((rest, value)) => {
                        if rest.input_len() == input.input_len() {
                            return Err(Err::Error(E::from_error_kind(input, ErrorKind::Many1)));
                        }
                        input = rest;
                        values.push(value);
                    }
                    Err(Err::Error(_)) => return Ok((input, values)),
                    Err(e) => return Err(e),
                }
            }
        }
    }
    pub fn many_m_n<
        I: Clone + InputLength,
        O,
        E: ParseError<I>,
        F: FnMut(I) -> IResult<I, O, E>,
    >(
        min: usize,
        max: usize,
        mut f: F,
    ) -> impl FnMut(I) -> IResult<I, Vec<O>, E> {
        move |mut input| {
            let mut values = Vec::new();
            for n in 0..max {
                match f(input.clone()) {
                    Ok((rest, value)) => {
                        if rest.input_len() == input.input_len() {
                            return Err(Err::Error(E::from_error_kind(input, ErrorKind::ManyMN)));
                        }
                        input = rest;
                        values.push(value);
                    }
                    Err(Err::Error(e)) if n < min => {
                        return Err(Err::Error(E::append(input, ErrorKind::ManyMN, e)))
                    }
                    Err(Err::Error(_)) => break,
                    Err(e) => return Err(e),
                }
            }
            Ok((input, values))
        }
    }
    pub fn separated_list0<
        I: Clone + InputLength,
        O,
        P,
        E: ParseError<I>,
        F: FnMut(I) -> IResult<I, P, E>,
        G: FnMut(I) -> IResult<I, O, E>,
    >(
        mut sep: F,
        mut f: G,
    ) -> impl FnMut(I) -> IResult<I, Vec<O>, E> {
        move |mut input| {
            let mut values = Vec::new();
            match f(input.clone()) {
                Ok((rest, value)) => {
                    input = rest;
                    values.push(value);
                }
                Err(Err::Error(_)) => return Ok((input, values)),
                Err(e) => return Err(e),
            }
            loop {
                let rest = match sep(input.clone()) {
                    Ok((rest, _)) => {
                        if rest.input_len() == input.input_len() {
                            return Err(Err::Error(E::from_error_kind(
                                input,
                                ErrorKind::SeparatedList,
                            )));
                        }
                        rest
                    }
                    Err(Err::Error(_)) => return Ok((input, values)),
                    Err(e) => return Err(e),
                };
                match f(rest) {
                    Ok((rest, value)) => {
                        input = rest;
                        values.push(value);
                    }
                    Err(Err::Error(_)) => return Ok((input, values)),
                    Err(e) => return Err(e),
                }
            }
        }
    }
}
pub mod bytes {
    pub mod complete {
        use super::super::*;
        pub fn tag<'a>(tag: &'static str) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
            move |input| {
                if input.starts_with(tag.as_bytes()) {
                    Ok((&input[tag.len()..], &input[..tag.len()]))
                } else {
                    Err(Err::Error(error::Error::new(input, ErrorKind::Tag)))
                }
            }
        }
        pub fn is_not<'a>(stop: [u8; 2]) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
            move |input| {
                let n = input
                    .iter()
                    .position(|b| stop.contains(b))
                    .unwrap_or(input.len());
                if n == 0 {
                    Err(Err::Error(error::Error::new(input, ErrorKind::IsNot)))
                } else {
                    Ok((&input[n..], &input[..n]))
                }
            }
        }
    }
}
pub mod character {
    pub mod complete {
        use super::super::*;
        pub fn char<'a>(c: char) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], char> {
            move |input| {
                if input.first().copied() == Some(c as u8) {
                    Ok((&input[1..], c))
                } else {
                    Err(Err::Error(error::Error::new(input, ErrorKind::Char)))
                }
            }
        }
        pub fn one_of<'a>(chars: &'static str) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], char> {
            move |input| match input.first() {
                Some(&c) if chars.as_bytes().contains(&c) => Ok((&input[1..], c as char)),
                _ => Err(Err::Error(error::Error::new(input, ErrorKind::OneOf))),
            }
        }
    }
}
