use std::iter;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use rust_syntax::{
    Attribute, Error, Pat, PatIdent, Token, parenthesized,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    token,
};

pub fn path_to_single_string(path: &rust_syntax::Path) -> Option<String> {
    if path.leading_colon.is_some() {
        return None;
    }
    let mut it = path.segments.iter();
    let segment = it.next()?;
    if it.next().is_some() {
        // Multipart path
        return None;
    }
    if segment.arguments != rust_syntax::PathArguments::None {
        return None;
    }
    Some(segment.ident.to_string())
}

pub fn ident_to_type(ident: rust_syntax::Ident) -> rust_syntax::Type {
    let mut path = rust_syntax::Path {
        leading_colon: None,
        segments: Default::default(),
    };
    path.segments.push(rust_syntax::PathSegment {
        ident,
        arguments: Default::default(),
    });
    rust_syntax::Type::Path(rust_syntax::TypePath { attrs: Vec::new(), qself: None, path })
}

pub fn empty_type() -> rust_syntax::Type {
    rust_syntax::Type::Tuple(rust_syntax::TypeTuple {
        attrs: Vec::new(),
        paren_token: Default::default(),
        elems: Default::default(),
    })
}

pub fn type_tuple(elems: impl Iterator<Item = rust_syntax::Type>) -> rust_syntax::TypeTuple {
    let mut result = rust_syntax::TypeTuple {
        attrs: Vec::new(),
        paren_token: Default::default(),
        elems: elems.collect(),
    };
    if !result.elems.empty_or_trailing() {
        result.elems.push_punct(Default::default());
    }
    result
}

pub fn empty_type_tuple() -> rust_syntax::TypeTuple {
    rust_syntax::TypeTuple {
        attrs: Vec::new(),
        paren_token: Default::default(),
        elems: Default::default(),
    }
}

pub fn modify_types_generics_hack<F>(ty_generics: &rust_syntax::TypeGenerics, mut mutator: F) -> rust_syntax::AngleBracketedGenericArguments
where
    F: FnMut(&mut rust_syntax::punctuated::Punctuated<rust_syntax::GenericArgument, rust_syntax::token::Comma>),
{
    let mut abga: rust_syntax::AngleBracketedGenericArguments =
        rust_syntax::parse2(ty_generics.to_token_stream()).unwrap_or_else(|_| rust_syntax::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: Default::default(),
            gt_token: Default::default(),
        });
    mutator(&mut abga.args);
    abga
}

pub fn strip_raw_ident_prefix(mut name: String) -> String {
    if name.starts_with("r#") {
        name.replace_range(0..2, "");
    }
    name
}

pub fn first_visibility(visibilities: &[Option<&rust_syntax::Visibility>]) -> proc_macro2::TokenStream {
    let vis = visibilities
        .iter()
        .flatten()
        .next()
        .expect("need at least one visibility in the list");

    vis.to_token_stream()
}

pub fn public_visibility() -> rust_syntax::Visibility {
    rust_syntax::Visibility::Public(rust_syntax::token::Pub::default())
}

pub fn expr_to_lit_string(expr: &rust_syntax::Expr) -> Result<String, Error> {
    match expr {
        rust_syntax::Expr::Lit(lit) => match &lit.lit {
            rust_syntax::Lit::Str(str) => Ok(str.value()),
            _ => Err(Error::new_spanned(expr, "attribute only allows str values")),
        },
        _ => Err(Error::new_spanned(expr, "attribute only allows str values")),
    }
}

#[allow(clippy::large_enum_variant)]
pub enum AttrArg {
    Flag(Ident),
    KeyValue(KeyValue),
    Sub(SubAttr),
    Not { not: Token![!], name: Ident },
    Fn(rust_syntax::ItemFn),
}

impl AttrArg {
    pub fn name(&self) -> &Ident {
        match self {
            AttrArg::Flag(name) => name,
            AttrArg::KeyValue(KeyValue { name, .. }) => name,
            AttrArg::Sub(SubAttr { name, .. }) => name,
            AttrArg::Not { name, .. } => name,
            AttrArg::Fn(func) => &func.sig.ident,
        }
    }

    pub fn incorrect_type(&self) -> rust_syntax::Error {
        let message = match self {
            AttrArg::Flag(name) => format!("{:?} is not supported as a flag", name.to_string()),
            AttrArg::KeyValue(KeyValue { name, .. }) => format!("{:?} is not supported as key-value", name.to_string()),
            AttrArg::Sub(SubAttr { name, .. }) => format!("{:?} is not supported as nested attribute", name.to_string()),
            AttrArg::Not { name, .. } => format!("{:?} cannot be nullified", name.to_string()),
            AttrArg::Fn(func) => format!("{:?} is not supported as a function", func.sig.ident.to_string()),
        };
        rust_syntax::Error::new_spanned(self, message)
    }

    pub fn flag(self) -> rust_syntax::Result<Ident> {
        if let Self::Flag(name) = self {
            Ok(name)
        } else {
            Err(self.incorrect_type())
        }
    }

    pub fn key_value(self) -> rust_syntax::Result<KeyValue> {
        if let Self::KeyValue(key_value) = self {
            Ok(key_value)
        } else {
            Err(self.incorrect_type())
        }
    }

    pub fn key_value_or_not(self) -> rust_syntax::Result<Option<KeyValue>> {
        match self {
            Self::KeyValue(key_value) => Ok(Some(key_value)),
            Self::Not { .. } => Ok(None),
            _ => Err(self.incorrect_type()),
        }
    }

    pub fn sub_attr(self) -> rust_syntax::Result<SubAttr> {
        if let Self::Sub(sub_attr) = self {
            Ok(sub_attr)
        } else {
            Err(self.incorrect_type())
        }
    }

    pub fn apply_flag_to_field(self, field: &mut Option<Span>, caption: &str) -> rust_syntax::Result<()> {
        match self {
            AttrArg::Flag(flag) => {
                if field.is_none() {
                    *field = Some(flag.span());
                    Ok(())
                } else {
                    Err(Error::new(
                        flag.span(),
                        format!("Illegal setting - field is already {caption}"),
                    ))
                }
            }
            AttrArg::Not { .. } => {
                *field = None;
                Ok(())
            }
            _ => Err(self.incorrect_type()),
        }
    }

    pub fn apply_potentialy_empty_sub_to_field<T: ApplyMeta>(
        self,
        field: &mut Option<T>,
        caption: &str,
        init: impl FnOnce(Span) -> T,
    ) -> rust_syntax::Result<()> {
        match self {
            AttrArg::Sub(sub) => {
                if field.is_none() {
                    let mut value = init(sub.span());
                    value.apply_sub_attr(sub)?;
                    *field = Some(value);
                    Ok(())
                } else {
                    Err(Error::new(
                        sub.span(),
                        format!("Illegal setting - field is already {caption}"),
                    ))
                }
            }
            AttrArg::Flag(flag) => {
                if field.is_none() {
                    *field = Some(init(flag.span()));
                    Ok(())
                } else {
                    Err(Error::new(
                        flag.span(),
                        format!("Illegal setting - field is already {caption}"),
                    ))
                }
            }
            AttrArg::Not { .. } => {
                *field = None;
                Ok(())
            }
            _ => Err(self.incorrect_type()),
        }
    }
}

pub struct KeyValue {
    pub name: Ident,
    pub eq: Token![=],
    pub value: TokenStream,
}

impl KeyValue {
    pub fn parse_value<T: Parse>(self) -> rust_syntax::Result<T> {
        rust_syntax::parse2(self.value)
    }
}

impl ToTokens for KeyValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

impl Parse for KeyValue {
    fn parse(input: rust_syntax::parse::ParseStream) -> rust_syntax::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

pub struct SubAttr {
    pub name: Ident,
    pub paren: token::Paren,
    pub args: TokenStream,
}

impl SubAttr {
    pub fn args<T: Parse>(self) -> rust_syntax::Result<impl IntoIterator<Item = T>> {
        Punctuated::<T, Token![,]>::parse_terminated.parse2(self.args)
    }
    pub fn undelimited<T: Parse>(self) -> rust_syntax::Result<impl IntoIterator<Item = T>> {
        (|p: ParseStream| iter::from_fn(|| (!p.is_empty()).then(|| p.parse())).collect::<rust_syntax::Result<Vec<T>>>()).parse2(self.args)
    }
}

impl ToTokens for SubAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.paren.surround(tokens, |t| self.args.to_tokens(t));
    }
}

fn get_cursor_after_parsing<P: Parse + Spanned>(input: rust_syntax::parse::ParseBuffer) -> rust_syntax::Result<rust_syntax::buffer::Cursor> {
    let parse_attempt: P = input.parse()?;
    let cursor = input.cursor();
    if cursor.eof() || input.peek(Token![,]) {
        Ok(cursor)
    } else {
        Err(rust_syntax::Error::new(
            parse_attempt.span(),
            "does not end with comma or end of section",
        ))
    }
}

fn get_token_stream_up_to_cursor(input: rust_syntax::parse::ParseStream, cursor: rust_syntax::buffer::Cursor) -> rust_syntax::Result<TokenStream> {
    Ok(core::iter::from_fn(|| {
        if input.cursor() < cursor {
            input.parse::<TokenTree>().ok()
        } else {
            None
        }
    })
    .collect())
}

impl Parse for AttrArg {
    fn parse(input: rust_syntax::parse::ParseStream) -> rust_syntax::Result<Self> {
        // Check for standalone function first
        if input.peek(Token![fn]) {
            return Ok(Self::Fn(input.parse()?));
        }

        if input.peek(Token![!]) {
            Ok(Self::Not {
                not: input.parse()?,
                name: input.parse()?,
            })
        } else {
            let name = input.parse()?;
            if input.peek(Token![,]) || input.is_empty() {
                Ok(Self::Flag(name))
            } else if input.peek(token::Paren) {
                let args;
                Ok(Self::Sub(SubAttr {
                    name,
                    paren: parenthesized!(args in input),
                    args: args.parse()?,
                }))
            } else if input.peek(Token![=]) {
                Ok(Self::KeyValue(KeyValue {
                    name,
                    eq: input.parse()?,
                    value: {
                        let cursor = get_cursor_after_parsing::<rust_syntax::Type>(input.fork())
                            .or_else(|_| get_cursor_after_parsing::<rust_syntax::Expr>(input.fork()))?;
                        get_token_stream_up_to_cursor(input, cursor)?
                    },
                }))
            } else {
                Err(input.error("expected !<ident>, <ident>=<value>, <ident>(…), or fn"))
            }
        }
    }
}

impl ToTokens for AttrArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttrArg::Flag(flag) => flag.to_tokens(tokens),
            AttrArg::KeyValue(kv) => kv.to_tokens(tokens),
            AttrArg::Sub(sub) => sub.to_tokens(tokens),
            AttrArg::Not { not, name } => {
                not.to_tokens(tokens);
                name.to_tokens(tokens);
            }
            AttrArg::Fn(func) => func.to_tokens(tokens),
        }
    }
}

pub trait ApplyMeta {
    fn apply_meta(&mut self, expr: AttrArg) -> Result<(), Error>;

    fn apply_sub_attr(&mut self, sub_attr: SubAttr) -> rust_syntax::Result<()> {
        for arg in sub_attr.args()? {
            self.apply_meta(arg)?;
        }
        Ok(())
    }

    fn apply_subsections(&mut self, list: &rust_syntax::MetaList) -> rust_syntax::Result<()> {
        if list.tokens.is_empty() {
            return Err(rust_syntax::Error::new_spanned(list, "Expected builder(…)"));
        }

        let parser = rust_syntax::punctuated::Punctuated::<_, rust_syntax::token::Comma>::parse_terminated;
        let exprs = parser.parse2(list.tokens.clone())?;
        for expr in exprs {
            self.apply_meta(expr)?;
        }

        Ok(())
    }

    fn apply_attr(&mut self, attr: &Attribute) -> rust_syntax::Result<()> {
        match &attr.meta {
            rust_syntax::Meta::List(list) => self.apply_subsections(list),
            meta => Err(Error::new_spanned(meta, "Expected builder(…)")),
        }
    }
}

pub fn pat_to_ident(i: usize, pat: &Pat) -> Ident {
    if let Pat::Ident(PatIdent { ident, .. }) = pat {
        ident.clone()
    } else {
        format_ident!("__{i}", span = pat.span())
    }
}

pub fn phantom_data_for_generics(generics: &rust_syntax::Generics) -> proc_macro2::TokenStream {
    let phantom_generics = generics.params.iter().filter_map(|param| match param {
        rust_syntax::GenericParam::Lifetime(lifetime) => {
            let lifetime = &lifetime.lifetime;
            Some(quote!(&#lifetime ()))
        }
        rust_syntax::GenericParam::Type(ty) => {
            let ty = &ty.ident;
            Some(ty.to_token_stream())
        }
        rust_syntax::GenericParam::Const(_cnst) => None,
    });
    quote!(::core::marker::PhantomData<(#( ::core::marker::PhantomData<#phantom_generics> ),*)>)
}
