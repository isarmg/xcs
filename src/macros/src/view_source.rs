//! Source identifiers and literal text used by the project CSR macros.
//!
//! Derived from leptos_hot_reload 0.8.7 (MIT), Greg Johnston and contributors.
//! Original identity and file receipts: compliance/provenance/origins.json.
//! Only the pure helpers consumed by view-macros are retained here.

use std::path::Path;

/// Preserve the upstream debug identifier, including non-UTF-8 fallback.
pub(crate) fn span_to_stable_id(path: impl AsRef<Path>, line: usize) -> String {
    let file = path
        .as_ref()
        .to_str()
        .unwrap_or_default()
        .replace(['/', '\\'], "-");
    format!("{file}-{line}")
}

/// Convert the literal forms accepted by component documentation.
pub(crate) fn value_to_string(value: &syn::Expr) -> Option<String> {
    match value {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Some(s.value()),
            syn::Lit::Char(c) => Some(c.value().to_string()),
            syn::Lit::Int(i) => Some(i.base10_digits().to_string()),
            syn::Lit::Float(f) => Some(f.base10_digits().to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Match upstream ASCII PascalCase component detection on the last path segment.
pub(crate) fn is_component_node(
    node: &rstml::node::NodeElement<impl rstml::node::CustomNode>,
) -> bool {
    match node.name() {
        rstml::node::NodeName::Path(path) => path.path.segments.last().is_some_and(|segment| {
            segment
                .ident
                .to_string()
                .starts_with(|c: char| c.is_ascii_uppercase())
        }),
        rstml::node::NodeName::Block(_) | rstml::node::NodeName::Punctuated(_) => false,
    }
}

#[cfg(test)]
#[path = "../../../test/macros/unit/view_source/tests.rs"]
mod tests;
