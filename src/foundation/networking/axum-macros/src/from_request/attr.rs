use crate::attr_parsing::{combine_attribute, parse_parenthesized_attribute, Combine};
use rust_syntax::{
    parse::{Parse, ParseStream},
    Token,
};

pub(crate) mod kw {
    rust_syntax::custom_keyword!(via);
    rust_syntax::custom_keyword!(rejection);
    rust_syntax::custom_keyword!(state);
}

#[derive(Default)]
pub(super) struct FromRequestContainerAttrs {
    pub(super) via: Option<(kw::via, rust_syntax::Path)>,
    pub(super) rejection: Option<(kw::rejection, rust_syntax::Path)>,
    pub(super) state: Option<(kw::state, rust_syntax::Type)>,
}

impl Parse for FromRequestContainerAttrs {
    fn parse(input: ParseStream<'_>) -> rust_syntax::Result<Self> {
        let mut via = None;
        let mut rejection = None;
        let mut state = None;

        while !input.is_empty() {
            let lh = input.lookahead1();
            if lh.peek(kw::via) {
                parse_parenthesized_attribute(input, &mut via)?;
            } else if lh.peek(kw::rejection) {
                parse_parenthesized_attribute(input, &mut rejection)?;
            } else if lh.peek(kw::state) {
                parse_parenthesized_attribute(input, &mut state)?;
            } else {
                return Err(lh.error());
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self {
            via,
            rejection,
            state,
        })
    }
}

impl Combine for FromRequestContainerAttrs {
    fn combine(mut self, other: Self) -> rust_syntax::Result<Self> {
        let Self {
            via,
            rejection,
            state,
        } = other;
        combine_attribute(&mut self.via, via)?;
        combine_attribute(&mut self.rejection, rejection)?;
        combine_attribute(&mut self.state, state)?;
        Ok(self)
    }
}

#[derive(Default)]
pub(super) struct FromRequestFieldAttrs {
    pub(super) via: Option<(kw::via, rust_syntax::Path)>,
}

impl Parse for FromRequestFieldAttrs {
    fn parse(input: ParseStream<'_>) -> rust_syntax::Result<Self> {
        let mut via = None;

        while !input.is_empty() {
            let lh = input.lookahead1();
            if lh.peek(kw::via) {
                parse_parenthesized_attribute(input, &mut via)?;
            } else {
                return Err(lh.error());
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self { via })
    }
}

impl Combine for FromRequestFieldAttrs {
    fn combine(mut self, other: Self) -> rust_syntax::Result<Self> {
        let Self { via } = other;
        combine_attribute(&mut self.via, via)?;
        Ok(self)
    }
}
