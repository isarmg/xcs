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
        let expression = rust_syntax::parse_str(source).unwrap();
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
        let expression = rust_syntax::parse_str(source).unwrap();
        assert_eq!(value_to_string(&expression), None, "{source}");
    }
}
