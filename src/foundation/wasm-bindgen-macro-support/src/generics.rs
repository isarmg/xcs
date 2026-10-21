use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use syn::parse_quote;
use syn::visit_mut::{self, VisitMut};
use syn::{visit::Visit, Ident, Type};

use crate::error::Diagnostic;

/// Visitor to replace wasm bindgen generics with their concrete types
/// The concrete type is the default type on the import if specified when it was defined.
struct GenericRenameVisitor<'a> {
    renames: &'a BTreeMap<&'a Ident, Option<Cow<'a, syn::Type>>>,
    err: Option<Diagnostic>,
}

impl<'a> VisitMut for GenericRenameVisitor<'a> {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if self.err.is_some() {
            return;
        }
        if let Type::Path(type_path) = ty {
            // Handle <T as Trait>::AssocType
            if let Some(qself) = &mut type_path.qself {
                if let Type::Path(qself_path) = &mut *qself.ty {
                    if qself_path.qself.is_none() && qself_path.path.segments.len() == 1 {
                        let ident = &qself_path.path.segments[0].ident;
                        if let Some((_, concrete)) = self.renames.get_key_value(ident) {
                            *qself.ty = if let Some(concrete) = concrete {
                                concrete.clone().into_owned()
                            } else {
                                parse_quote! { JsValue }
                            };
                            return;
                        }
                    }
                }
            }
            // Normal T::...
            if type_path.qself.is_none() && !type_path.path.segments.is_empty() {
                let first_seg = &type_path.path.segments[0];

                if let Some((_, concrete)) = self.renames.get_key_value(&first_seg.ident) {
                    if let Some(concrete) = concrete {
                        if type_path.path.segments.len() == 1 {
                            *ty = concrete.clone().into_owned();
                        } else if let Type::Path(concrete_path) = concrete.as_ref() {
                            let remaining: Vec<_> =
                                type_path.path.segments.iter().skip(1).cloned().collect();
                            type_path.path.segments = concrete_path.path.segments.clone();
                            type_path.path.segments.extend(remaining);
                        }
                    } else {
                        *ty = parse_quote! { JsValue };
                    }
                    return;
                }
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }
}

/// Helper visitor for generic parameter usage
#[derive(Debug)]
pub struct GenericNameVisitor<'a, 'b> {
    generic_params: &'a Vec<&'a Ident>,
    /// The generic params that were found
    found_set: &'b mut BTreeSet<Ident>,
}

/// Helper visitor for generic parameter usage
impl<'a, 'b> GenericNameVisitor<'a, 'b> {
    /// Construct a new generic name visitors with a param search set,
    /// and optionally a second parameter search set.
    pub fn new(generic_params: &'a Vec<&'a Ident>, found_set: &'b mut BTreeSet<Ident>) -> Self {
        Self {
            generic_params,
            found_set,
        }
    }

    fn generic_param(&self, ident: &Ident) -> Option<&Ident> {
        self.generic_params
            .iter()
            .copied()
            .find(|param| *param == ident)
    }

    fn record_generic_param(&mut self, ident: &Ident) -> bool {
        let Some(param) = self.generic_param(ident).cloned() else {
            return false;
        };
        // Keep the declaration's span when this identifier is later emitted as
        // a generated generic parameter, so rustc suggestions target the
        // declaration rather than an occurrence in a type argument.
        self.found_set.insert(param);
        true
    }
}

impl<'a, 'b> Visit<'a> for GenericNameVisitor<'a, 'b> {
    fn visit_type_reference(&mut self, type_ref: &'a syn::TypeReference) {
        if let syn::Type::Path(type_path) = &*type_ref.elem {
            // Handle <T as Trait>::AssocType - visit the qself type
            if let Some(qself) = &type_path.qself {
                syn::visit::visit_type(self, &qself.ty);
                // Also visit the path segments for any generic args
                for segment in &type_path.path.segments {
                    syn::visit::visit_path_segment(self, segment);
                }
                return;
            }

            if let Some(first_segment) = type_path.path.segments.first() {
                if type_path.path.segments.len() == 1 && first_segment.arguments.is_empty() {
                    if self.record_generic_param(&first_segment.ident) {
                        return;
                    }
                } else {
                    self.record_generic_param(&first_segment.ident);

                    syn::visit::visit_path_arguments(self, &first_segment.arguments);

                    for segment in type_path.path.segments.iter().skip(1) {
                        syn::visit::visit_path_segment(self, segment);
                    }
                    return;
                }
            }
        }

        // For other cases, continue normal visiting
        syn::visit::visit_type_reference(self, type_ref);
    }

    fn visit_path(&mut self, path: &'a syn::Path) {
        if let Some(first_segment) = path.segments.first() {
            self.record_generic_param(&first_segment.ident);
        }

        for segment in &path.segments {
            match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    for arg in &args.args {
                        match arg {
                            syn::GenericArgument::Type(ty) => {
                                syn::visit::visit_type(self, ty);
                            }
                            syn::GenericArgument::AssocType(binding) => {
                                // Don't visit binding.ident, only visit binding.ty
                                syn::visit::visit_type(self, &binding.ty);
                            }
                            _ => {
                                syn::visit::visit_generic_argument(self, arg);
                            }
                        }
                    }
                }
                syn::PathArguments::Parenthesized(args) => {
                    // Handle function syntax like FnMut(T) -> Result<R, JsValue>
                    for input in &args.inputs {
                        syn::visit::visit_type(self, &input.ty);
                    }
                    if let syn::ReturnType::Type(_, return_type) = &args.output {
                        syn::visit::visit_type(self, return_type);
                    }
                }
                syn::PathArguments::None => {}
            }
        }
    }
}

/// Obtain the generic parameters and their optional defaults
pub(crate) fn generic_params(generics: &syn::Generics) -> Vec<(&Ident, Option<&syn::Type>)> {
    generics
        .type_params()
        .map(|tp| (&tp.ident, tp.default.as_ref().map(|(_, ty)| ty)))
        .collect()
}

/// Returns a vector of token streams representing generic type parameters with their bounds.
/// For example, `<T: Clone, U: Display>` returns `[quote!(T: Clone), quote!(U: Display)]`.
/// This is useful for constructing impl blocks that need to add lifetimes while preserving bounds.
pub(crate) fn type_params_with_bounds(generics: &syn::Generics) -> Vec<proc_macro2::TokenStream> {
    generics
        .type_params()
        .map(|tp| {
            let ident = &tp.ident;
            let bounds = &tp.bounds;
            if bounds.is_empty() {
                quote::quote! { #ident }
            } else {
                quote::quote! { #ident: #bounds }
            }
        })
        .collect()
}

/// Returns lifetime declarations with their inline bounds. For example,
/// `<'a: 'b, 'b>` returns `[quote!('a: 'b), quote!('b)]`.
///
/// This does not include predicates from a `where` clause. Callers that emit
/// a new generic scope must carry those predicates separately.
pub(crate) fn lifetime_params_with_bounds(
    generics: &syn::Generics,
) -> Vec<proc_macro2::TokenStream> {
    generics
        .lifetimes()
        .map(|lp| {
            let lifetime = &lp.lifetime;
            let bounds = &lp.bounds;
            if bounds.is_empty() {
                quote::quote! { #lifetime }
            } else {
                quote::quote! { #lifetime: #bounds }
            }
        })
        .collect()
}

/// Obtain the generic bounds, both inline and where clauses together.
///
/// Inline bounds are reified into `where` predicates because a generated item
/// does not necessarily have a parameter-list slot to carry them: the wrapper
/// method declares only the parameters that were not hoisted onto its
/// enclosing `impl` block, so an inline bound on a hoisted parameter has
/// nowhere else to go. This covers *lifetime* parameters as well as type
/// parameters — an inline `<'a: 'b>` has no other carrier, and dropping it
/// makes the wrapper's declaration strictly weaker than the shim it calls
/// (which redeclares its lifetimes with bounds intact), i.e. an
/// `E0478`/"lifetime may not live long enough" against generated code.
pub(crate) fn generic_bounds<'a>(generics: &'a syn::Generics) -> Vec<Cow<'a, syn::WherePredicate>> {
    let mut bounds = Vec::new();
    for param in &generics.params {
        match param {
            syn::GenericParam::Type(type_param) if !type_param.bounds.is_empty() => {
                let ident = &type_param.ident;
                let predicate = syn::WherePredicate::Type(syn::PredicateType {
                    attrs: Vec::new(),
                    lifetimes: None,
                    bounded_ty: syn::parse_quote!(#ident),
                    colon_token: syn::Token![:](proc_macro2::Span::call_site()),
                    bounds: type_param.bounds.clone(),
                });
                bounds.push(Cow::Owned(predicate));
            }
            syn::GenericParam::Lifetime(lifetime_param) if !lifetime_param.bounds.is_empty() => {
                let predicate = syn::WherePredicate::Lifetime(syn::PredicateLifetime {
                    attrs: Vec::new(),
                    lifetime: lifetime_param.lifetime.clone(),
                    colon_token: syn::Token![:](proc_macro2::Span::call_site()),
                    bounds: lifetime_param.bounds.clone(),
                });
                bounds.push(Cow::Owned(predicate));
            }
            _ => {}
        }
    }
    if let Some(where_clause) = &generics.where_clause {
        bounds.extend(where_clause.predicates.iter().map(Cow::Borrowed));
    }
    bounds
}

/// Moves inline lifetime bounds to the `where` clause.
///
/// Rust accepts inline lifetime bounds in some input positions that
/// wasm-bindgen transforms into struct and impl declarations where only the
/// predicate form is accepted. Normalizing before emitting a new item keeps
/// every generated scope well-formed.
pub(crate) fn move_lifetime_bounds_to_where(generics: &mut syn::Generics) {
    let mut predicates = Vec::new();
    for param in &mut generics.params {
        let syn::GenericParam::Lifetime(param) = param else {
            continue;
        };
        if param.bounds.is_empty() {
            continue;
        }
        predicates.push(syn::WherePredicate::Lifetime(syn::PredicateLifetime {
            attrs: Vec::new(),
            lifetime: param.lifetime.clone(),
            colon_token: syn::Token![:](proc_macro2::Span::call_site()),
            bounds: std::mem::take(&mut param.bounds),
        }));
        param.colon_token = None;
    }
    if !predicates.is_empty() {
        generics.make_where_clause().predicates.extend(predicates);
    }
}

/// Replace specified lifetime parameters with 'static.
/// This is used when generating concrete ABI types for extern blocks,
/// which cannot have lifetime parameters from the outer scope.
/// Only the lifetimes in `lifetimes_to_staticize` are replaced.
pub(crate) fn staticize_lifetimes(
    mut ty: syn::Type,
    lifetimes_to_staticize: &[&syn::Lifetime],
) -> syn::Type {
    struct LifetimeStaticizer<'a> {
        lifetimes: &'a [&'a syn::Lifetime],
    }
    impl VisitMut for LifetimeStaticizer<'_> {
        fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
            if self.lifetimes.iter().any(|lt| lt.ident == lifetime.ident) {
                *lifetime = syn::Lifetime::new("'static", lifetime.span());
            }
        }
    }
    LifetimeStaticizer {
        lifetimes: lifetimes_to_staticize,
    }
    .visit_type_mut(&mut ty);
    ty
}

/// Obtain the generic type parameter names
pub(crate) fn generic_param_names(generics: &syn::Generics) -> Vec<&Ident> {
    generics.type_params().map(|tp| &tp.ident).collect()
}

/// Obtain lifetime arguments/names from generics.
///
/// These are valid in type argument positions, but do not carry the inline
/// bounds required when declaring an `impl` or function generic header.
pub(crate) fn lifetime_args(generics: &syn::Generics) -> Vec<&syn::Lifetime> {
    generics.lifetimes().map(|lp| &lp.lifetime).collect()
}

/// Generate a lifetime name that does not collide with the source generics.
pub(crate) fn fresh_lifetime(generics: &syn::Generics, base: &str) -> syn::Lifetime {
    let mut suffix = 0;
    loop {
        let name = if suffix == 0 {
            base.to_owned()
        } else {
            format!("{base}_{suffix}")
        };
        if generics
            .lifetimes()
            .all(|param| param.lifetime.ident != name)
        {
            return syn::Lifetime::new(&format!("'{name}"), proc_macro2::Span::call_site());
        }
        suffix += 1;
    }
}

/// Obtain both lifetime and type parameter names from generics
pub(crate) fn all_param_names(generics: &syn::Generics) -> (Vec<&syn::Lifetime>, Vec<&Ident>) {
    (lifetime_args(generics), generic_param_names(generics))
}

/// Helper visitor for lifetime usage detection in types
pub struct LifetimeVisitor<'a> {
    lifetime_params: &'a [&'a syn::Lifetime],
    found_set: BTreeSet<syn::Lifetime>,
    bound_lifetime_names: Vec<Ident>,
}

impl<'a> LifetimeVisitor<'a> {
    pub fn new(lifetime_params: &'a [&'a syn::Lifetime]) -> Self {
        Self {
            lifetime_params,
            found_set: BTreeSet::new(),
            bound_lifetime_names: Vec::new(),
        }
    }

    pub fn into_found(self) -> BTreeSet<syn::Lifetime> {
        self.found_set
    }

    fn push_bound_lifetimes(&mut self, lifetimes: Option<&syn::BoundLifetimes>) -> usize {
        let start = self.bound_lifetime_names.len();
        if let Some(lifetimes) = lifetimes {
            self.bound_lifetime_names
                .extend(lifetimes.lifetimes.iter().filter_map(|param| match param {
                    syn::GenericParam::Lifetime(param) => Some(param.lifetime.ident.clone()),
                    _ => None,
                }));
        }
        start
    }
}

impl<'ast> syn::visit::Visit<'ast> for LifetimeVisitor<'_> {
    fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
        if !self
            .bound_lifetime_names
            .iter()
            .rev()
            .any(|ident| ident == &lifetime.ident)
            && self.lifetime_params.contains(&lifetime)
        {
            self.found_set.insert(lifetime.clone());
        }
    }

    fn visit_bound_lifetimes(&mut self, lifetimes: &'ast syn::BoundLifetimes) {
        let start = self.push_bound_lifetimes(Some(lifetimes));
        syn::visit::visit_bound_lifetimes(self, lifetimes);
        self.bound_lifetime_names.truncate(start);
    }

    fn visit_predicate_type(&mut self, predicate: &'ast syn::PredicateType) {
        let start = self.push_bound_lifetimes(predicate.lifetimes.as_ref());
        syn::visit::visit_predicate_type(self, predicate);
        self.bound_lifetime_names.truncate(start);
    }

    fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
        let start = self.push_bound_lifetimes(bound.lifetimes.as_ref());
        syn::visit::visit_trait_bound(self, bound);
        self.bound_lifetime_names.truncate(start);
    }

    fn visit_type_fn_ptr(&mut self, fn_ptr: &'ast syn::TypeFnPtr) {
        let start = self.push_bound_lifetimes(fn_ptr.lifetimes.as_ref());
        syn::visit::visit_type_fn_ptr(self, fn_ptr);
        self.bound_lifetime_names.truncate(start);
    }
}

/// Find all lifetimes from the given set that are used in a type
pub(crate) fn used_lifetimes_in_type<'a>(
    ty: &syn::Type,
    lifetime_params: &'a [&'a syn::Lifetime],
) -> BTreeSet<syn::Lifetime> {
    let mut visitor = LifetimeVisitor::new(lifetime_params);
    syn::visit::Visit::visit_type(&mut visitor, ty);
    visitor.into_found()
}

/// Find all lifetimes from the given set that are used in a where predicate.
pub(crate) fn used_lifetimes_in_predicate<'a>(
    predicate: &syn::WherePredicate,
    lifetime_params: &'a [&'a syn::Lifetime],
) -> BTreeSet<syn::Lifetime> {
    let mut visitor = LifetimeVisitor::new(lifetime_params);
    syn::visit::Visit::visit_where_predicate(&mut visitor, predicate);
    visitor.into_found()
}

pub(crate) fn uses_generic_params(ty: &syn::Type, generic_names: &Vec<&Ident>) -> bool {
    let mut found_set = Default::default();
    let mut visitor = GenericNameVisitor::new(generic_names, &mut found_set);
    visitor.visit_type(ty);
    !found_set.is_empty()
}

/// Visitor that detects a reference to a generic type parameter appearing
/// anywhere within a type, including when nested inside other types (e.g.
/// `&T`, `&&T`, `Option<&T>`, `(T, &T)`, `[&T; N]`, `Box<&T>`).
struct RefToGenericVisitor<'a> {
    generic_params: &'a Vec<&'a Ident>,
    found: bool,
}

impl<'a, 'ast> Visit<'ast> for RefToGenericVisitor<'a> {
    fn visit_type_reference(&mut self, type_ref: &'ast syn::TypeReference) {
        // A reference whose referent mentions a generic type parameter would
        // require impls that don't generally exist (e.g. a higher-ranked
        // `for<'a> &'a T: IntoWasmAbi`), so flag it.
        if uses_generic_params(&type_ref.elem, self.generic_params) {
            self.found = true;
        }
        // Keep recursing to catch further-nested references.
        syn::visit::visit_type_reference(self, type_ref);
    }
}

/// Returns `true` if `ty` contains a reference (`&_`) whose referent mentions
/// one of the given generic type parameters, at any nesting depth.
pub(crate) fn references_generic_param(ty: &syn::Type, generic_names: &Vec<&Ident>) -> bool {
    let mut visitor = RefToGenericVisitor {
        generic_params: generic_names,
        found: false,
    };
    visitor.visit_type(ty);
    visitor.found
}

/// Visitor that detects whether an `impl Trait` type appears anywhere within
/// a type, including when nested inside other types (e.g. `&impl Trait`,
/// `Vec<impl Trait>`).
struct ImplTraitPresenceVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for ImplTraitPresenceVisitor {
    fn visit_type_impl_trait(&mut self, i: &'ast syn::TypeImplTrait) {
        self.found = true;
        // Keep recursing in case a bound itself nests another `impl Trait`
        // (not legal today, but cheap to keep general).
        syn::visit::visit_type_impl_trait(self, i);
    }
}

/// Returns `true` if `ty` contains an `impl Trait` type at any nesting depth.
///
/// Argument-position `impl Trait` desugars to an anonymous generic type
/// parameter, so it never shows up in `fn.generics` — callers that need to
/// know whether a signature is "generic" at all (e.g. deciding whether a
/// block-level `experimental_generic_mono` applies) have to check argument types
/// directly rather than `fn.generics.type_params()`.
pub(crate) fn has_impl_trait(ty: &syn::Type) -> bool {
    let mut visitor = ImplTraitPresenceVisitor { found: false };
    visitor.visit_type(ty);
    visitor.found
}

/// Visitor that rewrites every `impl Trait` type it finds (including nested,
/// e.g. `Vec<impl Trait>`) into a synthesized, named type parameter, pushing
/// one [`syn::GenericParam::Type`] per occurrence — carrying the `impl
/// Trait`'s own bounds — onto `params`.
///
/// Argument-position `impl Trait` desugars to an anonymous generic type
/// parameter that never appears in `fn.generics`, so there is no name for
/// `experimental_generic_mono` codegen to hang a `where` bound, shim generic
/// parameter, or `breaks_if_inlined::<..>` turbofish argument off of. This
/// visitor "un-desugars" it back into the named form a user would otherwise
/// have to write by hand (`fn f<T: Trait>(x: T)`), giving each occurrence a
/// synthesized, hygienic name so the rest of per-mono codegen can treat it
/// exactly like any other type parameter.
struct ImplTraitDesugar<'a> {
    params: &'a mut Vec<syn::GenericParam>,
}

impl<'a> VisitMut for ImplTraitDesugar<'a> {
    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        let syn::Type::ImplTrait(impl_trait) = ty else {
            // Keep recursing to reach a nested `impl Trait` (e.g. inside
            // `Vec<impl Trait>` or a tuple).
            visit_mut::visit_type_mut(self, ty);
            return;
        };
        // Spanned at the original `impl Trait` so a bound that fails to hold
        // (e.g. `T: IntoWasmAbi`) points back at the user's own type rather
        // than at generated code.
        let ident = Ident::new(
            &format!("__WasmBindgenImplTrait{}", self.params.len()),
            syn::spanned::Spanned::span(&*impl_trait),
        );
        self.params.push(syn::GenericParam::Type(syn::TypeParam {
            attrs: Vec::new(),
            ident: ident.clone(),
            colon_token: Some(Default::default()),
            bounds: impl_trait.bounds.clone(),
            default: None,
        }));
        *ty = syn::Type::Path(syn::TypePath {
            attrs: Vec::new(),
            qself: None,
            path: ident.into(),
        });
        // The replacement is a bare type-parameter path, which has nothing
        // left to recurse into.
    }
}

/// Rewrites every `impl Trait` type appearing within `ty` (at any nesting
/// depth) into a synthesized, named type parameter, appending one generic
/// parameter per occurrence to `params`. `ty` is left unchanged if it
/// contains no `impl Trait`.
///
/// `params` is threaded across every argument of a single function so that
/// synthesized names stay unique across the whole signature; pass the same
/// `Vec` for each call.
pub(crate) fn desugar_impl_trait(ty: &mut syn::Type, params: &mut Vec<syn::GenericParam>) {
    let mut visitor = ImplTraitDesugar { params };
    visitor.visit_type_mut(ty);
}

pub(crate) fn uses_lifetime_params(ty: &syn::Type, lifetime_params: &[&syn::Lifetime]) -> bool {
    !used_lifetimes_in_type(ty, lifetime_params).is_empty()
}

pub(crate) fn used_generic_params<'a>(
    ty: &'a syn::Type,
    generic_names: &'a Vec<&Ident>,
    mut used_params: BTreeSet<Ident>,
) -> BTreeSet<Ident> {
    let mut visitor = GenericNameVisitor::new(generic_names, &mut used_params);
    visitor.visit_type(ty);
    used_params
}

/// Checks whether every occurrence of each ident in `generic_names` that appears
/// anywhere inside `args` is in a *structurally constraining* position —
/// i.e. a position from which rustc can read the parameter off of the
/// constructed type. This mirrors the E0207 rule for type parameters on
/// `impl` blocks: a parameter must appear in `Self` (or a trait ref) in a
/// structurally determined slot, otherwise Rust can't infer it at use sites.
///
/// Constraining positions (for a param appearing somewhere inside):
///   - Bare: `T`
///   - As a type argument of a nominal path `Foo<..., T, ...>` (recursive)
///   - Under pointers, references, arrays, slices, tuples, bare function types,
///     parens, and ordinary trait generic arguments (recursive)
///
/// Non-constraining positions:
///   - Under a QSelf / projection: `<T as Trait>::X` or `T::X`
///   - Inside `dyn Fn(T)` / `impl Fn(T)` parenthesized trait syntax.
///   - Inside an associated-type binding's RHS (those project through the
///     outer trait, so they are not injective).
///
/// Returns `true` if the args are safe to hoist (all occurrences constraining,
/// or no occurrences at all), `false` if any occurrence is non-constraining.
pub(crate) fn args_are_constraining_for(
    args: &syn::punctuated::Punctuated<syn::GenericArgument, syn::Token![,]>,
    generic_names: &[&Ident],
) -> bool {
    for arg in args {
        match arg {
            syn::GenericArgument::Type(ty) if !type_is_constraining_for(ty, generic_names) => {
                return false;
            }
            // Associated type bindings (`Trait<Item = T>`) project through the
            // outer trait, so any fn generics inside the RHS are behind a
            // projection — not constraining.
            syn::GenericArgument::AssocType(binding)
                if type_mentions_any(&binding.ty, generic_names) =>
            {
                return false;
            }
            // Anything else (lifetimes, consts, already-constraining types,
            // future arg kinds) doesn't disqualify the args.
            _ => {}
        }
    }
    true
}

/// A type is "constraining" for the fn generics it contains iff every
/// occurrence of any `generic_names` ident within it is in a constraining
/// position. See [`args_are_constraining_for`] for the rules.
pub(crate) fn type_is_constraining_for(ty: &syn::Type, generic_names: &[&Ident]) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            // QSelf -> projection like `<T as Trait>::Assoc`. Any fn generic
            // appearing anywhere inside is behind a projection.
            if type_path.qself.is_some() {
                return !type_mentions_any(ty, generic_names);
            }

            // Bare `T` where T is a fn generic: constraining.
            if type_path.path.segments.len() == 1 {
                let seg = &type_path.path.segments[0];
                if matches!(seg.arguments, syn::PathArguments::None)
                    && generic_names.contains(&&seg.ident)
                {
                    return true;
                }
            }

            // `T::Foo...` (multi-segment path whose head is a fn generic)
            // is a projection through the head's implicit trait — any fn
            // generic inside is non-constraining.
            if type_path.path.segments.len() > 1 {
                if let Some(first) = type_path.path.segments.first() {
                    if generic_names.contains(&&first.ident) {
                        return !type_mentions_any(ty, generic_names);
                    }
                }
            }

            // Nominal path `Foo<..args..>`: recurse into the last segment's
            // args. Leading segments (module path) don't carry generics that
            // mention fn params.
            for seg in &type_path.path.segments {
                match &seg.arguments {
                    syn::PathArguments::None => {}
                    syn::PathArguments::AngleBracketed(a) => {
                        if !args_are_constraining_for(&a.args, generic_names) {
                            return false;
                        }
                    }
                    syn::PathArguments::Parenthesized(p) => {
                        // `Fn(T) -> U` sugar: function-pointer-like,
                        // non-constraining.
                        for input in &p.inputs {
                            if type_mentions_any(&input.ty, generic_names) {
                                return false;
                            }
                        }
                        if let syn::ReturnType::Type(_, ret) = &p.output {
                            if type_mentions_any(ret, generic_names) {
                                return false;
                            }
                        }
                    }
                }
            }
            true
        }
        syn::Type::Ptr(p) => type_is_constraining_for(&p.elem, generic_names),
        syn::Type::Reference(r) => type_is_constraining_for(&r.elem, generic_names),
        syn::Type::Array(a) => type_is_constraining_for(&a.elem, generic_names),
        syn::Type::Slice(s) => type_is_constraining_for(&s.elem, generic_names),
        syn::Type::Group(g) => type_is_constraining_for(&g.elem, generic_names),
        syn::Type::Paren(p) => type_is_constraining_for(&p.elem, generic_names),
        syn::Type::Tuple(t) => t
            .elems
            .iter()
            .all(|e| type_is_constraining_for(e, generic_names)),
        syn::Type::FnPtr(f) => {
            f.inputs
                .iter()
                .all(|input| type_is_constraining_for(&input.ty, generic_names))
                && match &f.output {
                    syn::ReturnType::Default => true,
                    syn::ReturnType::Type(_, ty) => type_is_constraining_for(ty, generic_names),
                }
        }
        syn::Type::TraitObject(object) => object.bounds.iter().all(|bound| match bound {
            syn::TypeParamBound::Trait(bound) => {
                bound
                    .path
                    .segments
                    .iter()
                    .all(|segment| match &segment.arguments {
                        syn::PathArguments::None => true,
                        syn::PathArguments::AngleBracketed(arguments) => {
                            args_are_constraining_for(&arguments.args, generic_names)
                        }
                        syn::PathArguments::Parenthesized(arguments) => {
                            !arguments
                                .inputs
                                .iter()
                                .any(|input| type_mentions_any(&input.ty, generic_names))
                                && match &arguments.output {
                                    syn::ReturnType::Default => true,
                                    syn::ReturnType::Type(_, ty) => {
                                        !type_mentions_any(ty, generic_names)
                                    }
                                }
                        }
                    })
            }
            syn::TypeParamBound::Lifetime(_) => true,
            _ => false,
        }),
        // ImplTrait / Infer / Never / Macro and future forms are handled
        // conservatively when they mention a function generic.
        _ => !type_mentions_any(ty, generic_names),
    }
}

/// Whether `ty` mentions any of the given idents anywhere (constraining or not).
fn type_mentions_any(ty: &syn::Type, generic_names: &[&Ident]) -> bool {
    let vec: Vec<&Ident> = generic_names.to_vec();
    let mut found = BTreeSet::new();
    let mut visitor = GenericNameVisitor::new(&vec, &mut found);
    visitor.visit_type(ty);
    !found.is_empty()
}

/// Usage visitor for generic bounds
pub(crate) fn generics_predicate_uses(
    predicate: &syn::WherePredicate,
    generic_names: &Vec<&Ident>,
) -> bool {
    let mut found_set = Default::default();
    let mut visitor = GenericNameVisitor::new(generic_names, &mut found_set);
    visitor.visit_where_predicate(predicate);
    !found_set.is_empty()
}

/// Concrete type replacement visitor application.
/// Replaces generic type parameters with their concrete types (or JsValue if no default),
/// and replaces specified lifetime parameters with 'static (since extern blocks cannot have
/// lifetime parameters from the outer scope).
pub(crate) fn generic_to_concrete<'a>(
    mut ty: syn::Type,
    generic_names: &BTreeMap<&'a Ident, Option<Cow<'a, syn::Type>>>,
    lifetimes_to_staticize: &[&syn::Lifetime],
) -> Result<syn::Type, Diagnostic> {
    // First, replace type parameters with their concrete types
    if !generic_names.is_empty() {
        let mut visitor = GenericRenameVisitor {
            renames: generic_names,
            err: None,
        };
        visitor.visit_type_mut(&mut ty);
        if let Some(err) = visitor.err {
            return Err(err);
        }
    }
    // Then, replace specified lifetimes with 'static for ABI compatibility
    Ok(staticize_lifetimes(ty, lifetimes_to_staticize))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/wasm-bindgen-macro-support/unit/src/generics/tests_796.rs"));
