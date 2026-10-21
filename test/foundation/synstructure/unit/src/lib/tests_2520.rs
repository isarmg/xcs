#[cfg(test)]
mod tests {
    use super::*;

    // Regression test for #48
    #[test]
    fn test_each_enum() {
        let di: syn::DeriveInput = syn::parse_quote! {
         enum A {
             Foo(usize, bool),
             Bar(bool, usize),
             Baz(usize, bool, usize),
             Quux(bool, usize, bool)
         }
        };
        let mut s = Structure::new(&di);

        s.filter(|bi| bi.ast().ty.to_token_stream().to_string() == "bool");

        assert_eq!(
            s.each(|bi| quote!(do_something(#bi))).to_string(),
            quote! {
                A::Foo(_, ref __binding_1,) => { { do_something(__binding_1) } }
                A::Bar(ref __binding_0, ..) => { { do_something(__binding_0) } }
                A::Baz(_, ref __binding_1, ..) => { { do_something(__binding_1) } }
                A::Quux(ref __binding_0, _, ref __binding_2,) => {
                    {
                        do_something(__binding_0)
                    }
                    {
                        do_something(__binding_2)
                    }
                }
            }
            .to_string()
        );
    }
}
