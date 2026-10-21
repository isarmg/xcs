use crate::internals::ast::{Container, Data};
use crate::internals::{attr, ungroup};
use proc_macro2::Span;
use std::collections::HashSet;
use rust_syntax::punctuated::{Pair, Punctuated};
use rust_syntax::Token;

// Remove the default from every type parameter because in the generated impls
// they look like associated types: "error: associated type bindings are not
// allowed here".
pub fn without_defaults(generics: &rust_syntax::Generics) -> rust_syntax::Generics {
    rust_syntax::Generics {
        params: generics
            .params
            .iter()
            .map(|param| match param {
                rust_syntax::GenericParam::Type(param) => rust_syntax::GenericParam::Type(rust_syntax::TypeParam {
                    default: None,
                    ..param.clone()
                }),
                _ => param.clone(),
            })
            .collect(),
        ..generics.clone()
    }
}

pub fn with_where_predicates(
    generics: &rust_syntax::Generics,
    predicates: &[rust_syntax::WherePredicate],
) -> rust_syntax::Generics {
    let mut generics = generics.clone();
    let dst_predicates = &mut generics.make_where_clause().predicates;

    for predicate in predicates {
        dst_predicates.push(predicate.clone());
    }
    generics
}

pub fn with_where_predicates_from_fields(
    cont: &Container,
    generics: &rust_syntax::Generics,
    from_field: fn(&attr::Field) -> Option<&[rust_syntax::WherePredicate]>,
) -> rust_syntax::Generics {
    let mut generics = generics.clone();
    let dst_predicates = &mut generics.make_where_clause().predicates;

    for field in cont.data.all_fields() {
        let Some(predicate_slice) = from_field(&field.attrs) else {
            continue;
        };
        for inner_predicate in predicate_slice {
            dst_predicates.push(inner_predicate.clone());
        }
    }
    generics
}

pub fn with_where_predicates_from_variants(
    cont: &Container,
    generics: &rust_syntax::Generics,
    from_variant: fn(&attr::Variant) -> Option<&[rust_syntax::WherePredicate]>,
) -> rust_syntax::Generics {
    let variants = match &cont.data {
        Data::Enum(variants) => variants,
        Data::Struct(_, _) => {
            return generics.clone();
        }
    };
    let mut generics = generics.clone();
    let dst_predicates = &mut generics.make_where_clause().predicates;

    for variant in variants {
        let Some(predicate_slice) = from_variant(&variant.attrs) else {
            continue;
        };
        for inner_predicate in predicate_slice {
            dst_predicates.push(inner_predicate.clone());
        }
    }
    generics
}

// Puts the given bound on any generic type parameters that are used in fields
// for which filter returns true.
//
// For example, the following struct needs the bound `A: Serialize, B:
// Serialize`.
//
//     struct S<'b, A, B: 'b, C> {
//         a: A,
//         b: Option<&'b B>
//         #[serialization(skip_serializing)]
//         c: C,
//     }
pub fn with_bound(
    cont: &Container,
    generics: &rust_syntax::Generics,
    filter: fn(&attr::Field, Option<&attr::Variant>) -> bool,
    bound: &rust_syntax::Path,
) -> rust_syntax::Generics {
    struct FindTyParams<'ast> {
        // Set of all generic type parameters on the current struct (A, B, C in
        // the example). Initialized up front.
        all_type_params: HashSet<rust_syntax::Ident>,

        // Set of generic type parameters used in fields for which filter
        // returns true (A and B in the example). Filled in as the visitor sees
        // them.
        relevant_type_params: HashSet<rust_syntax::Ident>,

        // Fields whose type is an associated type of one of the generic type
        // parameters.
        associated_type_usage: Vec<&'ast rust_syntax::TypePath>,
    }

    impl<'ast> FindTyParams<'ast> {
        fn visit_field(&mut self, field: &'ast rust_syntax::Field) {
            if let rust_syntax::Type::Path(ty) = ungroup(&field.ty) {
                if let Some(Pair::Punctuated(t, _)) = ty.path.segments.pairs().next() {
                    if self.all_type_params.contains(&t.ident) {
                        self.associated_type_usage.push(ty);
                    }
                }
            }
            self.visit_type(&field.ty);
        }

        fn visit_path(&mut self, path: &'ast rust_syntax::Path) {
            if let Some(seg) = path.segments.last() {
                if seg.ident == "PhantomData" {
                    // Hardcoded exception, because PhantomData<T> implements
                    // Serialize and Deserialize whether or not T implements it.
                    return;
                }
            }
            if path.leading_colon.is_none() && path.segments.len() == 1 {
                let id = &path.segments[0].ident;
                if self.all_type_params.contains(id) {
                    self.relevant_type_params.insert(id.clone());
                }
            }
            for segment in &path.segments {
                self.visit_path_segment(segment);
            }
        }

        // Everything below is simply traversing the syntax tree.

        fn visit_type(&mut self, ty: &'ast rust_syntax::Type) {
            match ty {
                #![cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
                rust_syntax::Type::Array(ty) => self.visit_type(&ty.elem),
                rust_syntax::Type::FnPtr(ty) => {
                    for arg in &ty.inputs {
                        self.visit_type(&arg.ty);
                    }
                    self.visit_return_type(&ty.output);
                }
                rust_syntax::Type::Group(ty) => self.visit_type(&ty.elem),
                rust_syntax::Type::ImplTrait(ty) => {
                    for bound in &ty.bounds {
                        self.visit_type_param_bound(bound);
                    }
                }
                rust_syntax::Type::Macro(ty) => self.visit_macro(&ty.mac),
                rust_syntax::Type::Paren(ty) => self.visit_type(&ty.elem),
                rust_syntax::Type::Path(ty) => {
                    if let Some(qself) = &ty.qself {
                        self.visit_type(&qself.ty);
                    }
                    self.visit_path(&ty.path);
                }
                rust_syntax::Type::Ptr(ty) => self.visit_type(&ty.elem),
                rust_syntax::Type::Reference(ty) => self.visit_type(&ty.elem),
                rust_syntax::Type::Slice(ty) => self.visit_type(&ty.elem),
                rust_syntax::Type::TraitObject(ty) => {
                    for bound in &ty.bounds {
                        self.visit_type_param_bound(bound);
                    }
                }
                rust_syntax::Type::Tuple(ty) => {
                    for elem in &ty.elems {
                        self.visit_type(elem);
                    }
                }

                rust_syntax::Type::Infer(_) | rust_syntax::Type::Never(_) | rust_syntax::Type::Verbatim(_) => {}

                _ => {}
            }
        }

        fn visit_path_segment(&mut self, segment: &'ast rust_syntax::PathSegment) {
            self.visit_path_arguments(&segment.arguments);
        }

        fn visit_path_arguments(&mut self, arguments: &'ast rust_syntax::PathArguments) {
            match arguments {
                rust_syntax::PathArguments::None => {}
                rust_syntax::PathArguments::AngleBracketed(arguments) => {
                    for arg in &arguments.args {
                        match arg {
                            #![cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
                            rust_syntax::GenericArgument::Type(arg) => self.visit_type(arg),
                            rust_syntax::GenericArgument::AssocType(arg) => self.visit_type(&arg.ty),
                            rust_syntax::GenericArgument::Lifetime(_)
                            | rust_syntax::GenericArgument::Const(_)
                            | rust_syntax::GenericArgument::AssocConst(_)
                            | rust_syntax::GenericArgument::Constraint(_) => {}
                            _ => {}
                        }
                    }
                }
                rust_syntax::PathArguments::Parenthesized(arguments) => {
                    for argument in &arguments.inputs {
                        self.visit_type(&argument.ty);
                    }
                    self.visit_return_type(&arguments.output);
                }
            }
        }

        fn visit_return_type(&mut self, return_type: &'ast rust_syntax::ReturnType) {
            match return_type {
                rust_syntax::ReturnType::Default => {}
                rust_syntax::ReturnType::Type(_, output) => self.visit_type(output),
            }
        }

        fn visit_type_param_bound(&mut self, bound: &'ast rust_syntax::TypeParamBound) {
            match bound {
                #![cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
                rust_syntax::TypeParamBound::Trait(bound) => self.visit_path(&bound.path),
                rust_syntax::TypeParamBound::Lifetime(_)
                | rust_syntax::TypeParamBound::PreciseCapture(_)
                | rust_syntax::TypeParamBound::Verbatim(_) => {}
                _ => {}
            }
        }

        // Type parameter should not be considered used by a macro path.
        //
        //     struct TypeMacro<T> {
        //         mac: T!(),
        //         marker: PhantomData<T>,
        //     }
        fn visit_macro(&mut self, _mac: &'ast rust_syntax::Macro) {}
    }

    let all_type_params = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();

    let mut visitor = FindTyParams {
        all_type_params,
        relevant_type_params: HashSet::new(),
        associated_type_usage: Vec::new(),
    };
    match &cont.data {
        Data::Enum(variants) => {
            for variant in variants {
                for field in &variant.fields {
                    if filter(&field.attrs, Some(&variant.attrs)) {
                        visitor.visit_field(field.original);
                    }
                }
            }
        }
        Data::Struct(_, fields) => {
            for field in fields {
                if filter(&field.attrs, None) {
                    visitor.visit_field(field.original);
                }
            }
        }
    }

    let relevant_type_params = visitor.relevant_type_params;
    let associated_type_usage = visitor.associated_type_usage;

    fn make_where_bounded_type(
        bounded_ty: rust_syntax::TypePath,
        bound: &rust_syntax::Path,
    ) -> rust_syntax::WherePredicate {
        rust_syntax::WherePredicate::Type(rust_syntax::PredicateType {
            attrs: Vec::new(),
            lifetimes: None,
            // the type parameter that is being bounded e.g. T
            bounded_ty: rust_syntax::Type::Path(bounded_ty),
            colon_token: <Token![:]>::default(),
            // the bound e.g. Serialize
            bounds: {
                let mut punct = Punctuated::new();
                punct.push(rust_syntax::TypeParamBound::Trait(rust_syntax::TraitBound {
                    paren_token: None,
                    lifetimes: None,
                    modifiers: rust_syntax::TraitBoundModifiers::default(),
                    maybe: None,
                    path: bound.clone(),
                }));
                punct
            },
        })
    }

    let mut dst_generics = generics.clone();
    let dst_predicates = &mut dst_generics.make_where_clause().predicates;
    for param in generics.type_params() {
        let id = &param.ident;
        if !relevant_type_params.contains(id) {
            continue;
        }
        let bounded_ty = rust_syntax::TypePath {
            attrs: Vec::new(),
            qself: None,
            path: id.clone().into(),
        };
        dst_predicates.push(make_where_bounded_type(bounded_ty, bound));
    }
    for bounded_ty in associated_type_usage {
        dst_predicates.push(make_where_bounded_type(bounded_ty.clone(), bound));
    }
    dst_generics
}

pub fn with_self_bound(
    cont: &Container,
    generics: &rust_syntax::Generics,
    bound: &rust_syntax::Path,
) -> rust_syntax::Generics {
    let mut generics = generics.clone();
    generics
        .make_where_clause()
        .predicates
        .push(rust_syntax::WherePredicate::Type(rust_syntax::PredicateType {
            attrs: Vec::new(),
            lifetimes: None,
            // the type that is being bounded e.g. MyStruct<'a, T>
            bounded_ty: type_of_item(cont),
            colon_token: <Token![:]>::default(),
            // the bound e.g. Default
            bounds: vec![rust_syntax::TypeParamBound::Trait(rust_syntax::TraitBound {
                paren_token: None,
                lifetimes: None,
                modifiers: rust_syntax::TraitBoundModifiers::default(),
                maybe: None,
                path: bound.clone(),
            })]
            .into_iter()
            .collect(),
        }));
    generics
}

pub fn with_lifetime_bound(generics: &rust_syntax::Generics, lifetime: &str) -> rust_syntax::Generics {
    let bound = rust_syntax::Lifetime::new(lifetime, Span::call_site());
    let def = rust_syntax::LifetimeParam {
        attrs: Vec::new(),
        lifetime: bound.clone(),
        colon_token: None,
        bounds: Punctuated::new(),
    };

    let params = Some(rust_syntax::GenericParam::Lifetime(def))
        .into_iter()
        .chain(generics.params.iter().cloned().map(|mut param| {
            match &mut param {
                rust_syntax::GenericParam::Lifetime(param) => {
                    param.bounds.push(bound.clone());
                }
                rust_syntax::GenericParam::Type(param) => {
                    param
                        .bounds
                        .push(rust_syntax::TypeParamBound::Lifetime(bound.clone()));
                }
                rust_syntax::GenericParam::Const(_) => {}
            }
            param
        }))
        .collect();

    rust_syntax::Generics {
        params,
        ..generics.clone()
    }
}

fn type_of_item(cont: &Container) -> rust_syntax::Type {
    rust_syntax::Type::Path(rust_syntax::TypePath {
        attrs: Vec::new(),
        qself: None,
        path: rust_syntax::Path {
            leading_colon: None,
            segments: vec![rust_syntax::PathSegment {
                ident: cont.ident.clone(),
                arguments: rust_syntax::PathArguments::AngleBracketed(
                    rust_syntax::AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: <Token![<]>::default(),
                        args: cont
                            .generics
                            .params
                            .iter()
                            .map(|param| match param {
                                rust_syntax::GenericParam::Type(param) => {
                                    rust_syntax::GenericArgument::Type(rust_syntax::Type::Path(rust_syntax::TypePath {
                                        attrs: Vec::new(),
                                        qself: None,
                                        path: param.ident.clone().into(),
                                    }))
                                }
                                rust_syntax::GenericParam::Lifetime(param) => {
                                    rust_syntax::GenericArgument::Lifetime(param.lifetime.clone())
                                }
                                rust_syntax::GenericParam::Const(_) => {
                                    panic!("Serialization does not support const generics yet");
                                }
                            })
                            .collect(),
                        gt_token: <Token![>]>::default(),
                    },
                ),
            }]
            .into_iter()
            .collect(),
        },
    })
}
