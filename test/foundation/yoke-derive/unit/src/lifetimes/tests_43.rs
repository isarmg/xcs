#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use rust_syntax::{parse_quote, Ident};

    use super::{custom_lt, replace_lifetime};

    fn a_ident() -> Ident {
        Ident::new("a", Span::call_site())
    }

    fn b_ident() -> Ident {
        Ident::new("b", Span::call_site())
    }

    #[test]
    fn replace_lt_static() {
        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(Foo<'static, T>), custom_lt("'a")),
            parse_quote!(Foo<'static, T>),
        );

        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(Foo<'a, T>), custom_lt("'static")),
            parse_quote!(Foo<'static, T>),
        );
    }

    #[test]
    fn replace_lt_ab() {
        // Probably shouldn't happen in actual use cases, but still works
        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(Foo<'a, 'b, T>), custom_lt("'c")),
            parse_quote!(Foo<'c, 'b, T>),
        );

        assert_eq!(
            replace_lifetime(
                &b_ident(),
                &parse_quote!(for<'a> fn(fn(Foo<'a, 'b, T>))),
                custom_lt("'c"),
            ),
            parse_quote!(for<'a> fn(fn(Foo<'a, 'c, T>))),
        );
    }

    #[test]
    fn replace_lt_macro() {
        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(foo!(Foo<'a, T>)), custom_lt("'b")),
            parse_quote!(foo!(Foo<'a, T>)),
        );

        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(Foo<'a, foo!(T)>), custom_lt("'b")),
            parse_quote!(Foo<'b, foo!(T)>),
        );

        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(Foo<foo!('a), T>), custom_lt("'b")),
            parse_quote!(Foo<foo!('a), T>),
        );
    }

    #[test]
    fn replace_raw() {
        assert_eq!(
            replace_lifetime(&a_ident(), &parse_quote!(Foo<'r#a, T>), custom_lt("'b")),
            parse_quote!(Foo<'b, T>),
        );
    }
}
