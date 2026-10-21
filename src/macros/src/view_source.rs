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
mod tests {
    use super::*;

    #[test]
    fn debug_ids_preserve_path_spelling_and_line_boundaries() {
        for (path, line, expected) in [
            ("src/web/view.rs", 1, "src-web-view.rs-1"),
            (r"C:\src\view.rs", 0, "C:-src-view.rs-0"),
            ("./视图/é.rs", 42, ".-视图-é.rs-42"),
            ("../a//b.rs", 9, "..-a--b.rs-9"),
            ("", usize::MAX, "-18446744073709551615"),
        ] {
            assert_eq!(span_to_stable_id(path, line), expected);
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_debug_path_keeps_upstream_empty_file_fallback() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        assert_eq!(
            span_to_stable_id(Path::new(OsStr::from_bytes(b"a\xff.rs")), 7),
            "-7"
        );
    }

    #[test]
    fn literal_text_preserves_escapes_unicode_and_numeric_normalization() {
        for (source, expected) in [
            (r#""a\n中文""#, "a\n中文"),
            (r##"r#"a\n"#"##, r"a\n"),
            ("'界'", "界"),
            ("'\\n'", "\n"),
            ("0xffu32", "255"),
            ("1_024usize", "1024"),
            ("1.25f32", "1.25"),
        ] {
            let expression = syn::parse_str(source).unwrap();
            assert_eq!(
                value_to_string(&expression).as_deref(),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn component_detection_preserves_ascii_case_and_last_segment_rules() {
        for (source, expected) in [
            ("<Button/>", true),
            ("<widget::Button/>", true),
            ("<Widget::button/>", false),
            ("<button/>", false),
            ("<élement/>", false),
            ("<my-tag/>", false),
            ("<{tag}/>", false),
        ] {
            let nodes = rstml::parse2(source.parse().unwrap()).unwrap();
            let rstml::node::Node::Element(element) = &nodes[0] else {
                panic!("expected element")
            };
            assert_eq!(is_component_node(element), expected, "{source}");
        }
    }

    #[test]
    fn unsupported_expressions_stay_unconverted() {
        for source in [
            "true",
            "false",
            "-12",
            "{\"text\"}",
            "(12)",
            "path::VALUE",
            "b\"text\"",
            "b'a'",
        ] {
            let expression = syn::parse_str(source).unwrap();
            assert_eq!(value_to_string(&expression), None, "{source}");
        }
    }
}
