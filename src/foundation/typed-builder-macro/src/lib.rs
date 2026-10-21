use proc_macro2::TokenStream;
use rust_syntax::{DeriveInput, parse::Error, parse_macro_input, spanned::Spanned};

mod builder_attr;
mod field_info;
mod mutator;
mod struct_info;
mod util;

#[proc_macro_derive(TypedBuilder, attributes(builder))]
pub fn derive_typed_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_my_derive(ast: &rust_syntax::DeriveInput) -> Result<TokenStream, Error> {
    let data = match &ast.data {
        rust_syntax::Data::Struct(data) => match &data.fields {
            rust_syntax::Fields::Named(fields) => struct_info::StructInfo::new(ast, fields.named.iter())?.derive()?,
            rust_syntax::Fields::Unnamed(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for tuple structs")),
            rust_syntax::Fields::Unit => return Err(Error::new(ast.span(), "TypedBuilder is not supported for unit structs")),
        },
        rust_syntax::Data::Enum(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for enums")),
        rust_syntax::Data::Union(_) => return Err(Error::new(ast.span(), "TypedBuilder is not supported for unions")),
    };
    Ok(data)
}
