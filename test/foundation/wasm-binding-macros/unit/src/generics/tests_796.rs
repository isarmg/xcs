#[cfg(test)]
mod tests {
    use quote::ToTokens;

    #[test]
    fn test_generic_name_visitor() {
        let t_ident = rust_syntax::Ident::new("T", proc_macro2::Span::call_site());
        let u_ident = rust_syntax::Ident::new("U", proc_macro2::Span::call_site());
        let generic_params = vec![&t_ident, &u_ident];

        // Test T as value
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(T);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));

        // Test &T as reference
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&T);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));

        // Test T<U> - both found
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(T<U>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));
        assert!(visitor.found_set.contains(&u_ident));

        // Test &T<U> - both found
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&T<U>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));
        assert!(visitor.found_set.contains(&u_ident));

        // Test T::<U>::Foo - T and U found, Foo ignored
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(T::<U>::Foo);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));
        assert!(visitor.found_set.contains(&u_ident));

        // Test Vec<T> - T found, Vec ignored
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(Vec<T>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));
        assert!(!visitor.found_set.contains(&u_ident));
    }

    #[test]
    fn test_associated_type_binding() {
        let t_ident = rust_syntax::Ident::new("T", proc_macro2::Span::call_site());
        let u_ident = rust_syntax::Ident::new("U", proc_macro2::Span::call_site());
        let generic_params = vec![&t_ident, &u_ident];

        // Test SomeTrait<T = U> - should find U (RHS) but NOT T (LHS assoc type name)
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(SomeTrait<T = U>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(!visitor.found_set.contains(&t_ident)); // T is LHS assoc type name
        assert!(visitor.found_set.contains(&u_ident)); // U is RHS generic parameter

        // Test SomeTrait<U = T> - should find T (RHS) but NOT U (LHS assoc type name)
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(SomeTrait<U = T>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident)); // T is RHS generic parameter
        assert!(!visitor.found_set.contains(&u_ident)); // U is LHS assoc type name
    }

    #[test]
    fn test_nested_references() {
        let t_ident = rust_syntax::Ident::new("T", proc_macro2::Span::call_site());
        let u_ident = rust_syntax::Ident::new("U", proc_macro2::Span::call_site());
        let generic_params = vec![&t_ident, &u_ident];

        // Test &T
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&T);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));

        // Test &&T
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&&T);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));

        // Test &&&T
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&&&T);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));

        // Test &T<U>
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&T<U>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));
        assert!(visitor.found_set.contains(&u_ident));
    }

    #[test]
    fn test_mixed_usage() {
        let t_ident = rust_syntax::Ident::new("T", proc_macro2::Span::call_site());
        let generic_params = vec![&t_ident];

        // Test T appearing in multiple places
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(SomeTrait<Item = T> + OtherTrait<Ref = &T>);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(visitor.found_set.contains(&t_ident));
    }

    #[test]
    fn test_ref_qself_trait_assoc_type() {
        let t_ident = rust_syntax::Ident::new("T", proc_macro2::Span::call_site());
        let generic_params = vec![&t_ident];

        // Test &<T as JsFunction1>::Arg1 - T should be found
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&<T as JsFunction1>::Arg1);
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);
        assert!(
            visitor.found_set.contains(&t_ident),
            "T should be found in &<T as JsFunction1>::Arg1"
        );
    }

    #[test]
    fn test_complex_reference_with_closure() {
        let t_ident = rust_syntax::Ident::new("T", proc_macro2::Span::call_site());
        let r_ident = rust_syntax::Ident::new("R", proc_macro2::Span::call_site());
        let generic_params = vec![&t_ident, &r_ident];

        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&Closure<dyn FnMut(T) -> Result<R, JsValue>>);

        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_type(&mut visitor, &ty);

        assert!(visitor.found_set.contains(&t_ident));
        assert!(visitor.found_set.contains(&r_ident));
    }

    #[test]
    fn test_generic_args_to_concrete() {
        use std::borrow::Cow;
        use std::collections::BTreeMap;

        // T -> String replacement
        let t = rust_syntax::parse_quote!(T);
        let str: rust_syntax::Type = rust_syntax::parse_quote!(String);
        let generic_names: BTreeMap<&rust_syntax::Ident, Option<Cow<rust_syntax::Type>>> = {
            let mut map = BTreeMap::new();
            map.insert(&t, Some(Cow::Borrowed(&str)));
            map
        };

        // T gets replaced with String
        let generic_type: rust_syntax::Type = rust_syntax::parse_quote!(Promise<T>);
        let result =
            crate::generics::generic_to_concrete(generic_type, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(Promise<String>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Mixed: i32 stays, T becomes String
        let mixed_type: rust_syntax::Type = rust_syntax::parse_quote!(Promise<i32, T>);
        let result = crate::generics::generic_to_concrete(mixed_type, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(Promise<i32, String>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // No generics to replace - unchanged
        let concrete_type: rust_syntax::Type = rust_syntax::parse_quote!(Promise<i32, bool>);
        let result =
            crate::generics::generic_to_concrete(concrete_type, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(Promise<i32, bool>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );
    }

    #[test]
    fn test_generic_associated_type_replacement() {
        use std::borrow::Cow;
        use std::collections::BTreeMap;

        let t: rust_syntax::Ident = rust_syntax::parse_quote!(T);
        let concrete: rust_syntax::Type = rust_syntax::parse_quote!(MyConcreteType);
        let generic_names: BTreeMap<&rust_syntax::Ident, Option<Cow<rust_syntax::Type>>> = {
            let mut map = BTreeMap::new();
            map.insert(&t, Some(Cow::Borrowed(&concrete)));
            map
        };

        // T::DurableObjectStub -> MyConcreteType::DurableObjectStub
        let assoc_type: rust_syntax::Type = rust_syntax::parse_quote!(T::DurableObjectStub);
        let result = crate::generics::generic_to_concrete(assoc_type, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(MyConcreteType::DurableObjectStub);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Nested: Vec<T::Item> -> Vec<MyConcreteType::Item>
        let nested: rust_syntax::Type = rust_syntax::parse_quote!(Vec<T::Item>);
        let result = crate::generics::generic_to_concrete(nested, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(Vec<MyConcreteType::Item>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Complex: WasmRet<<T::Stub as FromWasmAbi>::Abi>
        let complex: rust_syntax::Type = rust_syntax::parse_quote!(WasmRet<<T::Stub as FromWasmAbi>::Abi>);
        let result = crate::generics::generic_to_concrete(complex, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type =
            rust_syntax::parse_quote!(WasmRet<<MyConcreteType::Stub as FromWasmAbi>::Abi>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // T<Foo> gets fully replaced, args discarded
        let with_args: rust_syntax::Type = rust_syntax::parse_quote!(T<SomeArg>);
        let result = crate::generics::generic_to_concrete(with_args, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(MyConcreteType);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // QSelf: <T::DurableObjectStub as FromWasmAbi>::Abi
        let qself_type: rust_syntax::Type = rust_syntax::parse_quote!(<T::DurableObjectStub as FromWasmAbi>::Abi);
        let result = crate::generics::generic_to_concrete(qself_type, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type =
            rust_syntax::parse_quote!(<MyConcreteType::DurableObjectStub as FromWasmAbi>::Abi);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // QSelf with trait: <T as DurableObject>::DurableObjectStub
        let qself_trait: rust_syntax::Type = rust_syntax::parse_quote!(<T as DurableObject>::DurableObjectStub);
        let result =
            crate::generics::generic_to_concrete(qself_trait, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type =
            rust_syntax::parse_quote!(<MyConcreteType as DurableObject>::DurableObjectStub);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Reference to QSelf with trait: &<T as DurableObject>::DurableObjectStub
        let ref_qself_trait: rust_syntax::Type =
            rust_syntax::parse_quote!(&<T as DurableObject>::DurableObjectStub);
        let result =
            crate::generics::generic_to_concrete(ref_qself_trait, &generic_names, &[]).unwrap();
        let expected: rust_syntax::Type =
            rust_syntax::parse_quote!(&<MyConcreteType as DurableObject>::DurableObjectStub);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );
    }

    #[test]
    fn test_where_predicate_assoc_type_binding() {
        // Test that generics_predicate_uses finds generic params in associated type bindings
        // This is the pattern: F: JsFunction<Ret = Ret>
        // Both F and Ret should be detected as used

        let f_ident = rust_syntax::Ident::new("F", proc_macro2::Span::call_site());
        let ret_ident = rust_syntax::Ident::new("Ret", proc_macro2::Span::call_site());

        // Test with both F and Ret in the search set
        let generic_params = vec![&f_ident, &ret_ident];
        let predicate: rust_syntax::WherePredicate = rust_syntax::parse_quote!(F: JsFunction<Ret = Ret>);

        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_where_predicate(&mut visitor, &predicate);

        assert!(
            found_set.contains(&f_ident),
            "F should be found in 'F: JsFunction<Ret = Ret>'"
        );
        assert!(
            found_set.contains(&ret_ident),
            "Ret should be found in 'F: JsFunction<Ret = Ret>' (RHS of assoc type binding)"
        );
    }

    #[test]
    fn test_where_predicate_assoc_type_binding_only_rhs() {
        let f_ident = rust_syntax::Ident::new("F", proc_macro2::Span::call_site());
        let ret_ident = rust_syntax::Ident::new("Ret", proc_macro2::Span::call_site());

        // Ret in the search set
        let generic_params = vec![&ret_ident];
        let predicate: rust_syntax::WherePredicate = rust_syntax::parse_quote!(F: JsFunction<Ret = Ret>);

        let uses = crate::generics::generics_predicate_uses(&predicate, &generic_params);
        assert!(
            uses,
            "Ret should be detected as used in 'F: JsFunction<Ret = Ret>'"
        );

        // F in the search set
        let not_generic_params = vec![&f_ident];
        let uses = crate::generics::generics_predicate_uses(&predicate, &not_generic_params);
        assert!(
            uses,
            "F should not be detected as used in 'F: JsFunction<Ret = Ret>'"
        );
    }

    #[test]
    fn test_where_predicate_assoc_type_binding_only_bounded() {
        // Test that only F (not Ret) is found when Ret is not in the search set
        let f_ident = rust_syntax::Ident::new("F", proc_macro2::Span::call_site());
        let ret_ident = rust_syntax::Ident::new("Ret", proc_macro2::Span::call_site());

        // Only F in the search set
        let generic_params = vec![&f_ident];
        let predicate: rust_syntax::WherePredicate = rust_syntax::parse_quote!(F: JsFunction<Ret = Ret>);

        let uses = crate::generics::generics_predicate_uses(&predicate, &generic_params);
        assert!(
            uses,
            "F should be detected as used in 'F: JsFunction<Ret = Ret>'"
        );

        // Also verify Ret is NOT found when not in the search set
        let mut found_set = Default::default();
        let mut visitor = crate::generics::GenericNameVisitor::new(&generic_params, &mut found_set);
        rust_syntax::visit::visit_where_predicate(&mut visitor, &predicate);

        assert!(found_set.contains(&f_ident), "F should be found");
        assert!(
            !found_set.contains(&ret_ident),
            "Ret should NOT be found when not in search set"
        );
    }

    #[test]
    fn test_lifetime_params_with_bounds() {
        // No lifetimes -> empty
        let generics: rust_syntax::Generics = rust_syntax::parse_quote!(<T>);
        let result = crate::generics::lifetime_params_with_bounds(&generics);
        assert!(result.is_empty());

        // Unbounded lifetime
        let generics: rust_syntax::Generics = rust_syntax::parse_quote!(<'a, T>);
        let result = crate::generics::lifetime_params_with_bounds(&generics);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].to_string(), quote::quote!('a).to_string());

        // Bounded lifetime: 'a: 'b
        let generics: rust_syntax::Generics = rust_syntax::parse_quote!(<'a: 'b, 'b, T>);
        let result = crate::generics::lifetime_params_with_bounds(&generics);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].to_string(), quote::quote!('a: 'b).to_string());
        assert_eq!(result[1].to_string(), quote::quote!('b).to_string());
    }

    #[test]
    fn test_fresh_lifetime_avoids_source_names() {
        let generics: rust_syntax::Generics = rust_syntax::parse_quote!(<'__wbg_ref, '__wbg_ref_1, T>);
        let lifetime = crate::generics::fresh_lifetime(&generics, "__wbg_ref");
        assert_eq!(lifetime.to_string(), "'__wbg_ref_2");
    }

    #[test]
    fn test_lifetime_visitor_respects_hrtb_binders() {
        let outer_a: rust_syntax::Lifetime = rust_syntax::parse_quote!('a);
        let outer_b: rust_syntax::Lifetime = rust_syntax::parse_quote!('b);
        let lifetime_params = [&outer_a, &outer_b];

        let shadowing: rust_syntax::WherePredicate = rust_syntax::parse_quote!(for<'a> &'a T: Clone);
        assert!(
            crate::generics::used_lifetimes_in_predicate(&shadowing, &lifetime_params).is_empty()
        );

        let outer: rust_syntax::WherePredicate = rust_syntax::parse_quote!('a: 'b);
        let used = crate::generics::used_lifetimes_in_predicate(&outer, &lifetime_params);
        assert_eq!(used.len(), 2);
        assert!(used.contains(&outer_a));
        assert!(used.contains(&outer_b));
    }

    #[test]
    fn test_staticize_specific_lifetimes() {
        // Test that specified lifetimes in types are replaced with 'static
        let lifetime_a: rust_syntax::Lifetime = rust_syntax::parse_quote!('a);
        let lifetimes = [&lifetime_a];

        let ty: rust_syntax::Type = rust_syntax::parse_quote!(ScopedClosure<'a, dyn FnMut(T) -> R>);
        let result = crate::generics::staticize_lifetimes(ty, &lifetimes);
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(ScopedClosure<'static, dyn FnMut(T) -> R>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Test multiple lifetimes - only staticize specified ones
        let lifetime_b: rust_syntax::Lifetime = rust_syntax::parse_quote!('b);
        let lifetimes_both = [&lifetime_a, &lifetime_b];
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&'a SomeType<'b, T>);
        let result = crate::generics::staticize_lifetimes(ty, &lifetimes_both);
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(&'static SomeType<'static, T>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Test selective staticization - only 'a, not 'b
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(&'a SomeType<'b, T>);
        let result = crate::generics::staticize_lifetimes(ty, &[&lifetime_a]);
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(&'static SomeType<'b, T>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Test no lifetimes to staticize (should be unchanged)
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(Vec<T>);
        let result = crate::generics::staticize_lifetimes(ty, &[]);
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(Vec<T>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );
    }

    #[test]
    fn test_generic_to_concrete_with_lifetimes() {
        use std::borrow::Cow;
        use std::collections::BTreeMap;

        // Test that generic_to_concrete replaces both type params AND specified lifetimes
        let t: rust_syntax::Ident = rust_syntax::parse_quote!(T);
        let concrete: rust_syntax::Type = rust_syntax::parse_quote!(JsValue);
        let generic_names: BTreeMap<&rust_syntax::Ident, Option<Cow<rust_syntax::Type>>> = {
            let mut map = BTreeMap::new();
            map.insert(&t, Some(Cow::Borrowed(&concrete)));
            map
        };

        // Create the lifetime 'a that we want to staticize
        let lifetime_a: rust_syntax::Lifetime = rust_syntax::parse_quote!('a);
        let lifetimes_to_staticize = [&lifetime_a];

        // ScopedClosure<'a, dyn FnMut(T)> -> ScopedClosure<'static, dyn FnMut(JsValue)>
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(ScopedClosure<'a, dyn FnMut(T)>);
        let result =
            crate::generics::generic_to_concrete(ty, &generic_names, &lifetimes_to_staticize)
                .unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(ScopedClosure<'static, dyn FnMut(JsValue)>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );

        // Test that lifetimes NOT in the list are preserved
        let _lifetime_b: rust_syntax::Lifetime = rust_syntax::parse_quote!('b);
        let lifetimes_only_a = [&lifetime_a];
        let ty: rust_syntax::Type = rust_syntax::parse_quote!(Foo<'a, 'b>);
        let result =
            crate::generics::generic_to_concrete(ty, &BTreeMap::new(), &lifetimes_only_a).unwrap();
        let expected: rust_syntax::Type = rust_syntax::parse_quote!(Foo<'static, 'b>);
        assert_eq!(
            quote::quote!(#result).to_string(),
            quote::quote!(#expected).to_string()
        );
    }

    /// Parse a type whose last path segment carries generic args and hand them
    /// to `args_are_constraining_for`. This mirrors how `class_return_path()`
    /// feeds the gate.
    fn args_are_constraining(ty_src: &str, params: &[&str]) -> bool {
        let ty: rust_syntax::Type = rust_syntax::parse_str(ty_src).expect("valid type");
        let path = match ty {
            rust_syntax::Type::Path(rust_syntax::TypePath {
                attrs: _,
                qself: None,
                path,
            }) => path,
            _ => panic!("test helper expects a bare path type"),
        };
        let seg = path.segments.last().expect("at least one segment");
        let args = match &seg.arguments {
            rust_syntax::PathArguments::AngleBracketed(a) => a.args.clone(),
            rust_syntax::PathArguments::None => Default::default(),
            rust_syntax::PathArguments::Parenthesized(_) => {
                panic!("test helper doesn't handle paren-style args at the top")
            }
        };
        let idents: Vec<rust_syntax::Ident> = params
            .iter()
            .map(|p| rust_syntax::Ident::new(p, proc_macro2::Span::call_site()))
            .collect();
        let refs: Vec<&rust_syntax::Ident> = idents.iter().collect();
        crate::generics::args_are_constraining_for(&args, &refs)
    }

    #[test]
    fn hoist_gate_accepts_bare_idents() {
        // `Array<T>` — bare param, trivially constraining.
        assert!(args_are_constraining("Array<T>", &["T"]));
        // Multiple bare params.
        assert!(args_are_constraining("Map<K, V>", &["K", "V"]));
    }

    #[test]
    fn hoist_gate_accepts_nested_nominal() {
        // `Array<Option<T>>` — T is nested inside a nominal path, still
        // constraining. This was wrongly rejected by the old bare-ident gate.
        assert!(args_are_constraining("Array<Option<T>>", &["T"]));
        // Deeply nested.
        assert!(args_are_constraining("Array<Vec<Box<T>>>", &["T"]));
        // References, arrays, tuples preserve constraining-ness.
        assert!(args_are_constraining("Foo<&T>", &["T"]));
        assert!(args_are_constraining("Foo<[T; 4]>", &["T"]));
        assert!(args_are_constraining("Foo<(T, U)>", &["T", "U"]));
    }

    #[test]
    fn hoist_gate_accepts_when_param_absent() {
        // T doesn't appear at all → nothing to hoist → vacuously safe.
        assert!(args_are_constraining("Array<i32>", &["T"]));
        assert!(args_are_constraining("Promise<JsValue>", &["T"]));
    }

    #[test]
    fn hoist_gate_rejects_qself_projection() {
        // `Promise<<T as Promising>::Resolution>` — T only appears behind a
        // projection, which is NOT constraining. This is the shape that
        // produced E0207 before the fix.
        assert!(!args_are_constraining(
            "Promise<<T as Promising>::Resolution>",
            &["T"]
        ));
    }

    #[test]
    fn hoist_gate_rejects_bare_projection() {
        // `Array<T::Item>` — T appears as the head of a multi-segment path,
        // which Rust resolves through an implicit projection. Non-constraining.
        assert!(!args_are_constraining("Array<T::Item>", &["T"]));
        // Even if U is constraining, T's non-constraining presence disqualifies
        // the whole return path (partial hoisting would still be ill-formed).
        assert!(!args_are_constraining("Foo<T::Item, U>", &["T", "U"]));
    }

    #[test]
    fn hoist_gate_accepts_pointer_fn_and_trait_object_shapes() {
        assert!(args_are_constraining("Foo<*const T>", &["T"]));
        assert!(args_are_constraining("Foo<fn(T) -> i32>", &["T"]));
        assert!(args_are_constraining("Foo<Box<dyn Marker<T>>>", &["T"]));
    }

    #[test]
    fn hoist_gate_rejects_fn_sugar() {
        // Parenthesized `Fn(T) -> U` trait syntax is not treated as an ordinary
        // nominal generic argument list.
        assert!(!args_are_constraining("Foo<Box<dyn Fn(T) -> i32>>", &["T"]));
        // Return-position of the parenthesized sugar also counts.
        assert!(!args_are_constraining("Foo<Box<dyn Fn(i32) -> T>>", &["T"]));
    }

    #[test]
    fn hoist_gate_rejects_assoc_type_binding_rhs() {
        // `Trait<Item = T>` — T sits behind the outer trait's projection.
        assert!(!args_are_constraining(
            "Foo<Box<dyn Iterator<Item = T>>>",
            &["T"]
        ));
    }

    #[test]
    fn desugar_impl_trait_rewrites_top_level_and_nested() {
        use super::desugar_impl_trait;

        // Top-level `impl Trait` becomes a bare synthesized type-parameter path.
        let mut ty: rust_syntax::Type = rust_syntax::parse_quote!(impl Clone);
        let mut params = Vec::new();
        desugar_impl_trait(&mut ty, &mut params);
        assert_eq!(params.len(), 1);
        let rust_syntax::GenericParam::Type(tp) = &params[0] else {
            panic!("expected a synthesized type parameter");
        };
        assert_eq!(tp.bounds.to_token_stream().to_string(), "Clone");
        // The type was rewritten to a bare path naming the synthesized param.
        assert_eq!(ty.to_token_stream().to_string(), tp.ident.to_string());

        // Nested `impl Trait` (e.g. `Vec<impl Trait>`) is rewritten in place
        // without disturbing the rest of the type.
        let mut ty: rust_syntax::Type = rust_syntax::parse_quote!(Vec<impl Clone>);
        let mut params = Vec::new();
        desugar_impl_trait(&mut ty, &mut params);
        assert_eq!(params.len(), 1);
        let rust_syntax::GenericParam::Type(tp) = &params[0] else {
            panic!("expected a synthesized type parameter");
        };
        assert_eq!(
            ty.to_token_stream().to_string(),
            format!("Vec < {} >", tp.ident)
        );

        // A type with no `impl Trait` is left completely unchanged and no
        // params are synthesized.
        let mut ty: rust_syntax::Type = rust_syntax::parse_quote!(Vec<T>);
        let original = ty.to_token_stream().to_string();
        let mut params = Vec::new();
        desugar_impl_trait(&mut ty, &mut params);
        assert!(params.is_empty());
        assert_eq!(ty.to_token_stream().to_string(), original);

        // Multiple occurrences in one type each get a distinct name, and
        // names stay unique across repeated calls sharing one `params` list
        // (mirroring how one function's multiple arguments are processed).
        let mut ty: rust_syntax::Type = rust_syntax::parse_quote!((impl Clone, impl Iterator));
        let mut params = Vec::new();
        desugar_impl_trait(&mut ty, &mut params);
        assert_eq!(params.len(), 2);
        let names: Vec<String> = params
            .iter()
            .map(|p| match p {
                rust_syntax::GenericParam::Type(tp) => tp.ident.to_string(),
                _ => panic!("expected a type parameter"),
            })
            .collect();
        assert_ne!(names[0], names[1]);

        let mut ty2: rust_syntax::Type = rust_syntax::parse_quote!(impl Debug);
        desugar_impl_trait(&mut ty2, &mut params);
        assert_eq!(params.len(), 3);
        // The third synthesized name must not collide with the first two.
        let third = match &params[2] {
            rust_syntax::GenericParam::Type(tp) => tp.ident.to_string(),
            _ => panic!("expected a type parameter"),
        };
        assert!(!names.contains(&third));
    }
}
