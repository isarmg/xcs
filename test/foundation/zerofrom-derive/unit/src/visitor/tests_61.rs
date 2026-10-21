#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use std::collections::HashMap;
    use rust_syntax::{parse_quote, Ident};

    use super::check_type_for_parameters;
    fn make_typarams(params: &[&str]) -> HashMap<Ident, Option<Ident>> {
        params
            .iter()
            .map(|x| (Ident::new(x, Span::call_site()), None))
            .collect()
    }

    #[test]
    fn test_simple_type() {
        let environment = make_typarams(&["T", "U", "V"]);

        let ty = parse_quote!(Foo<'a, T>);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, true));

        let ty = parse_quote!(Foo<T>);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, false));

        let ty = parse_quote!(Foo<'static, T>);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, false));

        let ty = parse_quote!(Foo<'a>);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (false, true));

        let ty = parse_quote!(Foo<'a, Bar<U>, Baz<(V, u8)>>);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, true));

        let ty = parse_quote!(Foo<'a, W>);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (false, true));
    }

    #[test]
    fn test_assoc_types() {
        let environment = make_typarams(&["T"]);

        let ty = parse_quote!(<Foo as SomeTrait<'a, T>>::Output);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, true));

        let ty = parse_quote!(<Foo as SomeTrait<'static, T>>::Output);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, false));

        let ty = parse_quote!(<T as SomeTrait<'static, Foo>>::Output);
        let check = check_type_for_parameters(&ty, &environment);
        assert_eq!(check, (true, false));
    }
}
