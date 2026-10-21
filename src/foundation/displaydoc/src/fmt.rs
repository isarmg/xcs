use crate::attr::Display;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use rust_syntax::{Ident, LitStr};

macro_rules! peek_next {
    ($read:ident) => {
        match $read.chars().next() {
            Some(next) => next,
            None => return,
        }
    };
}

impl Display {
    // Transform `"error {var}"` to `"error {}", var`.
    pub(crate) fn expand_shorthand(&mut self) {
        let span = self.fmt.span();
        let fmt = self.fmt.value();
        let mut read = fmt.as_str();
        let mut out = String::new();
        let mut args = TokenStream::new();

        while let Some(brace) = read.find('{') {
            out += &read[..=brace];
            read = &read[brace + 1..];

            // skip cases where we find a {{
            if read.starts_with('{') {
                out.push('{');
                read = &read[1..];
                continue;
            }

            let next = peek_next!(read);

            let var = match next {
                '0'..='9' => take_int(&mut read),
                'a'..='z' | 'A'..='Z' | '_' => take_ident(&mut read),
                _ => return,
            };

            let ident = Ident::new(&var, span);

            let next = peek_next!(read);

            let arg = if cfg!(feature = "std") && next == '}' {
                quote_spanned!(span=> , #ident.__displaydoc_display())
            } else {
                quote_spanned!(span=> , #ident)
            };

            args.extend(arg);
        }

        out += read;
        self.fmt = LitStr::new(&out, self.fmt.span());
        self.args = args;
    }
}

fn take_int(read: &mut &str) -> String {
    let mut int = String::new();
    int.push('_');
    for (i, ch) in read.char_indices() {
        match ch {
            '0'..='9' => int.push(ch),
            _ => {
                *read = &read[i..];
                break;
            }
        }
    }
    int
}

fn take_ident(read: &mut &str) -> String {
    let mut ident = String::new();
    for (i, ch) in read.char_indices() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ident.push(ch),
            _ => {
                *read = &read[i..];
                break;
            }
        }
    }
    ident
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/displaydoc/unit/src/fmt/tests_91.rs"));
