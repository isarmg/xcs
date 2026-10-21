// $ cargo bench --features full,test --bench file

#![feature(rustc_private, test)]
#![recursion_limit = "1024"]
#![allow(
    clippy::elidable_lifetime_names,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::match_like_matches_macro,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_lifetimes,
    clippy::uninlined_format_args
)]

extern crate test;

#[macro_use]
#[path = "../../../../test/foundation/rust-syntax/tests/macros/mod.rs"]
mod macros;

#[allow(dead_code)]
#[path = "../../../../test/foundation/rust-syntax/tests/repo/mod.rs"]
mod repo;

use proc_macro2::{Span, TokenStream};
use std::fs;
use std::str::FromStr;
use rust_syntax::parse::{ParseStream, Parser};
use test::Bencher;

const FILE: &str = "../../../test/foundation/rust-syntax/tests/rust/library/core/src/str/mod.rs";

fn get_tokens() -> TokenStream {
    repo::clone_rust();
    let content = fs::read_to_string(FILE).unwrap();
    TokenStream::from_str(&content).unwrap()
}

#[bench]
fn baseline(b: &mut Bencher) {
    let tokens = get_tokens();
    b.iter(|| drop(tokens.clone()));
}

#[bench]
fn create_token_buffer(b: &mut Bencher) {
    let tokens = get_tokens();
    fn immediate_fail(_input: ParseStream) -> rust_syntax::Result<()> {
        Err(rust_syntax::Error::new(Span::call_site(), ""))
    }
    b.iter(|| immediate_fail.parse2(tokens.clone()));
}

#[bench]
fn parse_file(b: &mut Bencher) {
    let tokens = get_tokens();
    b.iter(|| rust_syntax::parse2::<rust_syntax::File>(tokens.clone()));
}
