//! Executes the maintained diagnostic handler, including abort recovery.
use proc_macro::TokenStream;
use proc_macro_error2::{abort_call_site, proc_macro_error};

#[proc_macro]
#[proc_macro_error]
pub fn checked_literal(input: TokenStream) -> TokenStream {
    if input.to_string() != "approved" {
        abort_call_site!("native diagnostic regression: expected approved");
    }
    "42_u32".parse().unwrap()
}
