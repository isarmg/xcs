//! Host-side implementation of the canonical wasm-bindgen macro protocol.

#[macro_use]
mod error;

mod ast;
mod codegen;
mod encode;
mod generics;
mod hash;
mod parser;

use codegen::TryToTokens;
use error::Diagnostic;
use parser::BindgenAttrs;
use parser::{ConvertToAst, MacroParse};
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use quote::TokenStreamExt;
use rust_syntax::parse::Parser;
use rust_syntax::parse::{Parse, ParseStream, Result as SyntaxResult};
use rust_syntax::Token;

fn cfg_gate_conditions(meta: &rust_syntax::Meta) -> Vec<TokenStream> {
    if meta.path().is_ident("cfg") {
        return match meta {
            rust_syntax::Meta::List(list) => vec![list.tokens.clone()],
            _ => Vec::new(),
        };
    }
    let rust_syntax::Meta::List(list) = meta else {
        return Vec::new();
    };
    if !list.path.is_ident("cfg_attr") {
        return Vec::new();
    }
    let Ok(args) =
        rust_syntax::punctuated::Punctuated::<rust_syntax::Meta, Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
    else {
        return Vec::new();
    };
    let mut args = args.iter();
    let Some(predicate) = args.next() else {
        return Vec::new();
    };
    args.flat_map(cfg_gate_conditions)
        .map(|condition| quote! { any(not(#predicate), #condition) })
        .collect()
}

pub(crate) fn cfg_gate_attrs(attrs: &[rust_syntax::Attribute]) -> Vec<rust_syntax::Attribute> {
    attrs
        .iter()
        .flat_map(|attr| cfg_gate_conditions(&attr.meta))
        .map(|condition| rust_syntax::parse_quote! { #[cfg(#condition)] })
        .collect()
}

/// Takes the parsed input from a `#[wasm_bindgen]` macro and returns the generated bindings
fn expand(attr: TokenStream, input: TokenStream) -> Result<TokenStream, Diagnostic> {
    parser::reset_attrs_used();
    // if struct is encountered, add `derive` attribute and let everything happen there (workaround
    // to help parsing cfg_attr correctly).
    let item = rust_syntax::parse2::<rust_syntax::Item>(input)?;
    if let rust_syntax::Item::Struct(mut s) = item {
        let opts: BindgenAttrs = rust_syntax::parse2(attr.clone())?;

        // If the emitted `#[derive(<path>::__rt::BindgenedStruct)]` below
        // fails to resolve — most commonly because `wasm-bindgen` is not a
        // direct dependency, so `::wasm_bindgen` is not in the extern
        // prelude — rustc strips the failed derive and, since
        // `#[wasm_bindgen(#attr)]` is then no longer an inert derive helper,
        // falls back to re-invoking this attribute macro on the struct with
        // identical input, recursing until the recursion limit. The emitted
        // `__wasm_bindgen_retried` marker attribute survives that round trip
        // (and is otherwise an inert helper of the derive), letting us detect
        // the re-invocation and report a proper error instead.
        if strip_retry_marker(&mut s.attrs) {
            bail_span!(
                s.ident,
                "cannot resolve a path to the `wasm_bindgen` crate: add `wasm-bindgen` \
                 as a direct dependency, or specify an explicit path with \
                 `#[wasm_bindgen(wasm_bindgen = path::to::wasm_bindgen)]`"
            );
        }
        let wasm_bindgen = opts
            .wasm_bindgen()
            .cloned()
            .unwrap_or_else(|| rust_syntax::parse_quote! { ::wasm_bindgen });

        // Inject `parent: <wasm_bindgen>::Parent<Parent>` when the struct
        // declares `#[wasm_bindgen(extends = Parent)]`, so users never write
        // the field themselves. Also rejects user-declared `Parent<T>` fields.
        let extends_path = opts.attrs.iter().find_map(|(_, a)| match a {
            parser::BindgenAttr::Extends(_, path) => Some(path.clone()),
            _ => None,
        });
        parser::inject_parent_field(&mut s, extends_path.as_ref(), &wasm_bindgen)?;

        let item = quote! {
            #[derive(#wasm_bindgen::__rt::BindgenedStruct)]
            #[wasm_bindgen(#attr)]
            #[__wasm_bindgen_retried]
            #s
        };
        return Ok(item);
    }

    let opts = rust_syntax::parse2(attr)?;
    let mut tokens = proc_macro2::TokenStream::new();
    let mut program = ast::Program::default();
    item.macro_parse(&mut program, (Some(opts), &mut tokens))?;
    program.try_to_tokens(&mut tokens)?;

    // If we successfully got here then we should have used up all attributes
    // and considered all of them to see if they were used. If one was forgotten
    // that's a bug on our end, so sanity check here.
    parser::check_unused_attrs(&mut tokens);

    Ok(tokens)
}

/// Finds and removes the `#[__wasm_bindgen_retried]` marker attribute left by
/// a previous expansion attempt whose emitted derive path failed to resolve.
fn strip_retry_marker(attrs: &mut Vec<rust_syntax::Attribute>) -> bool {
    let before = attrs.len();
    attrs.retain(|attr| !attr.path().is_ident("__wasm_bindgen_retried"));
    before != attrs.len()
}

/// Takes the parsed input from a `wasm_bindgen::link_to` macro and returns the generated link
fn expand_link_to(input: TokenStream) -> Result<TokenStream, Diagnostic> {
    parser::reset_attrs_used();
    let opts = rust_syntax::parse2(input)?;

    let mut tokens = proc_macro2::TokenStream::new();
    let link = parser::link_to(opts)?;
    link.try_to_tokens(&mut tokens)?;

    Ok(tokens)
}

/// Takes the parsed input from a `#[wasm_bindgen]` macro and returns the generated bindings
fn expand_class_marker(attr: TokenStream, input: TokenStream) -> Result<TokenStream, Diagnostic> {
    parser::reset_attrs_used();
    let mut item = rust_syntax::parse2::<rust_syntax::ImplItemFn>(input)?;
    let opts: ClassMarker = rust_syntax::parse2(attr)?;

    let mut program = ast::Program::default();
    item.macro_parse(&mut program, &opts)?;

    // This is where things are slightly different, we are being expanded in the
    // context of an impl so we can't inject arbitrary item-like tokens into the
    // output stream. If we were to do that then it wouldn't parse!
    //
    // Instead what we want to do is to generate the tokens for `program` into
    // the header of the function. This'll inject some no_mangle functions and
    // statics and such, and they should all be valid in the context of the
    // start of a function.
    //
    // We manually implement `ToTokens for ImplItemFn` here, injecting our
    // program's tokens before the actual method's inner body tokens.
    let mut tokens = proc_macro2::TokenStream::new();
    tokens.append_all(
        item.attrs
            .iter()
            .filter(|attr| matches!(attr.style, rust_syntax::AttrStyle::Outer)),
    );
    item.vis.to_tokens(&mut tokens);
    item.sig.to_tokens(&mut tokens);
    let mut err = None;
    item.block.brace_token.surround(&mut tokens, |tokens| {
        if let Err(e) = program.try_to_tokens(tokens) {
            err = Some(e);
        }
        parser::check_unused_attrs(tokens); // same as above
        tokens.append_all(
            item.attrs
                .iter()
                .filter(|attr| matches!(attr.style, rust_syntax::AttrStyle::Inner(_))),
        );
        tokens.append_all(&item.block.stmts);
    });

    if let Some(err) = err {
        return Err(err);
    }

    Ok(tokens)
}

struct ClassMarker {
    class: rust_syntax::Ident,
    js_class: String,
    js_namespace: Option<Vec<String>>,
    wasm_bindgen: rust_syntax::Path,
    js_sys: rust_syntax::Path,
}

impl Parse for ClassMarker {
    fn parse(input: ParseStream) -> SyntaxResult<Self> {
        let class = input.parse::<rust_syntax::Ident>()?;
        input.parse::<Token![=]>()?;
        let mut js_class = input.parse::<rust_syntax::LitStr>()?.value();
        js_class = js_class
            .strip_prefix("r#")
            .map(String::from)
            .unwrap_or(js_class);

        let mut js_namespace: Option<Vec<String>> = None;
        let mut wasm_bindgen = None;
        let mut js_sys = None;

        loop {
            if input.parse::<Option<Token![,]>>()?.is_some() {
                let ident = input.parse::<rust_syntax::Ident>()?;

                if ident == "js_namespace" {
                    if js_namespace.is_some() {
                        return Err(rust_syntax::Error::new(
                            ident.span(),
                            "found duplicate `js_namespace`",
                        ));
                    }
                    input.parse::<Token![=]>()?;
                    let content;
                    rust_syntax::bracketed!(content in input);
                    let segs: rust_syntax::punctuated::Punctuated<rust_syntax::LitStr, Token![,]> =
                        content.parse_terminated(
                            |p: ParseStream| p.parse::<rust_syntax::LitStr>(),
                            Token![,],
                        )?;
                    js_namespace = Some(segs.into_iter().map(|s| s.value()).collect());
                } else if ident == "wasm_bindgen" {
                    if wasm_bindgen.is_some() {
                        return Err(rust_syntax::Error::new(
                            ident.span(),
                            "found duplicate `wasm_bindgen`",
                        ));
                    }

                    input.parse::<Token![=]>()?;
                    wasm_bindgen = Some(input.parse::<rust_syntax::Path>()?);
                } else if ident == "js_sys" {
                    if js_sys.is_some() {
                        return Err(rust_syntax::Error::new(
                            ident.span(),
                            "found duplicate `js_sys`",
                        ));
                    }

                    input.parse::<Token![=]>()?;
                    js_sys = Some(input.parse::<rust_syntax::Path>()?);
                } else {
                    return Err(rust_syntax::Error::new(
                        ident.span(),
                        "expected `js_namespace`, `wasm_bindgen`, or `js_sys`",
                    ));
                }
            } else {
                break;
            }
        }

        Ok(ClassMarker {
            class,
            js_class,
            js_namespace,
            wasm_bindgen: wasm_bindgen
                .unwrap_or_else(|| rust_syntax::parse_quote! { wasm_bindgen }),
            js_sys: js_sys.unwrap_or_else(|| rust_syntax::parse_quote! { js_sys }),
        })
    }
}

fn expand_struct_marker(item: TokenStream) -> Result<TokenStream, Diagnostic> {
    parser::reset_attrs_used();

    let mut s: rust_syntax::ItemStruct = rust_syntax::parse2(item)?;

    let mut program = ast::Program::default();
    program.structs.push((&mut s).convert(&program)?);

    let mut tokens = proc_macro2::TokenStream::new();
    program.try_to_tokens(&mut tokens)?;

    parser::check_unused_attrs(&mut tokens);

    Ok(tokens)
}

extern crate proc_macro;

use proc_macro::TokenStream as MacroTokenStream;

/// A list of all the attributes can be found here: https://wasm-bindgen.github.io/wasm-bindgen/reference/attributes/index.html
#[proc_macro_attribute]
pub fn wasm_bindgen(attr: MacroTokenStream, input: MacroTokenStream) -> MacroTokenStream {
    match expand(attr.into(), input.into()) {
        Ok(tokens) => {
            if cfg!(xxx_debug_only_print_generated_code) {
                println!("{tokens}");
            }
            tokens.into()
        }
        Err(diagnostic) => (quote! { #diagnostic }).into(),
    }
}

/// This macro takes a JS module as input and returns a URL that can be used to
/// access it at runtime.
///
/// The module can be specified in a few ways:
/// - You can use `inline_js = "..."` to create an inline JS file.
/// - You can use `module = "/foo/bar"` to reference a file relative to the
///   root of the crate the macro is invoked in.
///
/// The returned URL can be used for things like creating workers/worklets:
/// ```no_run
/// use web_sys::Worker;
/// let worker = Worker::new(&wasm_bindgen::link_to!(module = "/src/worker.js"));
/// ```
#[proc_macro]
pub fn link_to(input: MacroTokenStream) -> MacroTokenStream {
    match expand_link_to(input.into()) {
        Ok(tokens) => {
            if cfg!(xxx_debug_only_print_generated_code) {
                println!("{tokens}");
            }
            tokens.into()
        }
        // This `String::clone` is here so that IDEs know this is supposed to be a
        // `String` and can keep type-checking the rest of the program even if the macro
        // fails.
        Err(diagnostic) => (quote! { String::clone(#diagnostic) }).into(),
    }
}

#[proc_macro_attribute]
pub fn __wasm_bindgen_class_marker(
    attr: MacroTokenStream,
    input: MacroTokenStream,
) -> MacroTokenStream {
    match expand_class_marker(attr.into(), input.into()) {
        Ok(tokens) => {
            if cfg!(xxx_debug_only_print_generated_code) {
                println!("{tokens}");
            }
            tokens.into()
        }
        Err(diagnostic) => (quote! { #diagnostic }).into(),
    }
}

#[proc_macro_derive(BindgenedStruct, attributes(wasm_bindgen, __wasm_bindgen_retried))]
pub fn __wasm_bindgen_struct_marker(item: MacroTokenStream) -> MacroTokenStream {
    match expand_struct_marker(item.into()) {
        Ok(tokens) => {
            if cfg!(xxx_debug_only_print_generated_code) {
                println!("{tokens}");
            }
            tokens.into()
        }
        Err(diagnostic) => (quote! { #diagnostic }).into(),
    }
}

#[cfg(test)]
#[path = "../../../../test/wasm-binding-macros/browser_futures.rs"]
mod browser_futures;
