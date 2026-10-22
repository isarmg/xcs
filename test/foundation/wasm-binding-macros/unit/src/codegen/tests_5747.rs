#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_replacement_does_not_recurse_into_its_arguments() {
        let mut predicate: rust_syntax::WherePredicate = rust_syntax::parse_quote!(A::Assoc: Trait<B>);
        let type_replacements = BTreeMap::from([
            (rust_syntax::parse_quote!(A), rust_syntax::parse_quote!(Projector<B>)),
            (rust_syntax::parse_quote!(B), rust_syntax::parse_quote!(T)),
        ]);

        assert!(substitute_class_predicate(
            &mut predicate,
            &type_replacements,
            &BTreeMap::new(),
        ));
        assert_eq!(
            predicate.to_token_stream().to_string(),
            "Projector < B > :: Assoc : Trait < T >"
        );
    }

    #[test]
    fn qualified_self_replacement_is_applied_once() {
        let mut predicate: rust_syntax::WherePredicate = rust_syntax::parse_quote!(<A as Trait>::Item: Bound<B>);
        let type_replacements = BTreeMap::from([
            (rust_syntax::parse_quote!(A), rust_syntax::parse_quote!(B)),
            (rust_syntax::parse_quote!(B), rust_syntax::parse_quote!(A)),
        ]);

        assert!(substitute_class_predicate(
            &mut predicate,
            &type_replacements,
            &BTreeMap::new(),
        ));
        assert_eq!(
            predicate.to_token_stream().to_string(),
            "< B as Trait > :: Item : Bound < A >"
        );
    }
}
