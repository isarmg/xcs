// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

//! Visitor for determining whether a type has type and non-static lifetime parameters
//! (duplicated in yoke/derive/src/visitor.rs)

use std::collections::HashMap;
use syn::visit::{visit_lifetime, visit_type, visit_type_path, Visit};
use syn::{Ident, Lifetime, Type, TypePath};

struct TypeVisitor<'a> {
    /// The type parameters in scope
    typarams: &'a HashMap<Ident, Option<Ident>>,
    /// Whether we found a type parameter
    found_typarams: bool,
    /// Whether we found a non-'static lifetime parameter
    found_lifetimes: bool,
}

impl<'ast> Visit<'ast> for TypeVisitor<'_> {
    fn visit_lifetime(&mut self, lt: &'ast Lifetime) {
        if lt.ident != "static" {
            self.found_lifetimes = true;
        }
        visit_lifetime(self, lt)
    }
    fn visit_type_path(&mut self, ty: &'ast TypePath) {
        // We only need to check ty.path.get_ident() and not ty.qself or any
        // generics in ty.path because the visitor will eventually visit those
        // types on its own
        if let Some(ident) = ty.path.get_ident() {
            if let Some(maybe_borrowed) = self.typarams.get(ident) {
                self.found_typarams = true;
                if maybe_borrowed.is_some() {
                    self.found_lifetimes = true;
                }
            }
        }

        visit_type_path(self, ty)
    }
}

/// Checks if a type has type or lifetime parameters, given the local context of
/// named type parameters. Returns `(has_type_params, has_lifetime_params)`
pub fn check_type_for_parameters(
    ty: &Type,
    typarams: &HashMap<Ident, Option<Ident>>,
) -> (bool, bool) {
    let mut visit = TypeVisitor {
        typarams,
        found_typarams: false,
        found_lifetimes: false,
    };
    visit_type(&mut visit, ty);

    (visit.found_typarams, visit.found_lifetimes)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerofrom-derive/unit/src/visitor/tests_61.rs"));
