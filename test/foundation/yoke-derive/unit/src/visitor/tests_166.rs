#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use std::collections::HashSet;
    use rust_syntax::{parse_quote, Ident};

    use super::{check_type_for_parameters, CheckResult};

    fn a_ident() -> Ident {
        Ident::new("a", Span::call_site())
    }

    fn yoke_ident() -> Ident {
        Ident::new("yoke", Span::call_site())
    }

    fn make_typarams(params: &[&str]) -> HashSet<Ident> {
        params
            .iter()
            .map(|x| Ident::new(x, Span::call_site()))
            .collect()
    }

    fn uses(lifetime: bool, typarams: bool) -> CheckResult {
        CheckResult {
            uses_lifetime_param: lifetime,
            uses_type_params: typarams,
            min_underscores_for_yoke_lt: 0,
        }
    }

    #[test]
    fn test_simple_type() {
        let environment = make_typarams(&["T", "U", "V"]);

        let ty = parse_quote!(Foo<'a, T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, true));

        let ty = parse_quote!(Foo<T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, true));

        let ty = parse_quote!(Foo<'static, T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, true));

        let ty = parse_quote!(Foo<'a>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, false));

        let ty = parse_quote!(Foo<'a, Bar<U>, Baz<(V, u8)>>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, true));

        let ty = parse_quote!(Foo<'a, W>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, false));
    }

    #[test]
    fn test_assoc_types() {
        let environment = make_typarams(&["T"]);

        let ty = parse_quote!(<Foo as SomeTrait<'a, T>>::Output);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, true));

        let ty = parse_quote!(<Foo as SomeTrait<'static, T>>::Output);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, true));

        let ty = parse_quote!(<T as SomeTrait<'static, Foo>>::Output);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, true));
    }

    #[test]
    fn test_macro_types() {
        let environment = make_typarams(&["T"]);

        // Macro types are opaque. Note that the environment doesn't contain `U` or `V`,
        // and the `T` is basically ignored.

        let ty = parse_quote!(foo!(Foo<'a, T>));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, false));

        let ty = parse_quote!(foo!(Foo<T>));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, false));

        let ty = parse_quote!(foo!(Foo<'static, T>));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, false));

        let ty = parse_quote!(foo!(Foo<'a>));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, false));

        let ty = parse_quote!(foo!(Foo<'a, Bar<U>, Baz<(V, u8)>>));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, false));

        let ty = parse_quote!(foo!(Foo<'a, W>));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, false));
    }

    #[test]
    fn test_raw_types() {
        let environment = make_typarams(&["T", "U", "V"]);

        let ty = parse_quote!(Foo<'a, r#T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, true));

        let ty = parse_quote!(Foo<r#T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, true));

        let ty = parse_quote!(Foo<'static, r#T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(false, true));

        let ty = parse_quote!(Foo<'a>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, false));

        let ty = parse_quote!(Foo<'a, Bar<r#U>, Baz<(r#V, u8)>>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, true));

        let ty = parse_quote!(Foo<'a, r#W>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check, uses(true, false));
    }

    #[test]
    fn test_yoke_lifetime() {
        let environment = make_typarams(&["T", "U", "V"]);

        let ty = parse_quote!(Foo<'yoke, r#T>);
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 0);

        let ty = parse_quote!(for<'yoke> fn(&'yoke ()));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 1);

        let ty = parse_quote!(for<'_yoke> fn(&'_yoke ()));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 2);

        let ty = parse_quote!(for<'_yoke_> fn(&'_yoke_ ()));
        let check = check_type_for_parameters(&yoke_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 0);

        let ty = parse_quote!(for<'_yoke, '___yoke> fn(&'_yoke (), &'___yoke ()));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 4);

        let ty = parse_quote!(for<'___yoke> fn(for<'_yoke> fn(&'_yoke (), &'___yoke ())));
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 4);

        let ty = parse_quote! {
            for<'yoke> fn(for<'_yoke> fn(for<'b> fn(&'b (), &'_yoke (), &'yoke ())))
        };
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 2);

        let ty = parse_quote! {
            for<'yoke> fn(for<'r#_yoke> fn(for<'b> fn(&'b (), &'_yoke (), &'yoke ())))
        };
        let check = check_type_for_parameters(&a_ident(), &environment, &ty);
        assert_eq!(check.min_underscores_for_yoke_lt, 2);
    }
}
