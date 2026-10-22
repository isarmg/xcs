use super::*;

fn previous_normalization(name: &Ident) -> Ident {
    let name_str = name.to_string();
    if name_str != name_str.to_case(Snake) {
        name.clone()
    } else {
        Ident::new(&name_str.to_case(Pascal), name.span())
    }
}

#[test]
fn component_names_preserve_conditional_pascal_conversion() {
    for (name, expected) in [
        ("snake_case", "SnakeCase"),
        ("lowercase", "Lowercase"),
        ("CamelCase", "CamelCase"),
        ("HTTPServer", "HTTPServer"),
        ("XMLHttpRequest", "XMLHttpRequest"),
        ("version2Value", "version2Value"),
        ("version_2_value", "Version2Value"),
        ("_private", "Private"),
        ("two__words", "TwoWords"),
        ("école_bleue", "ÉcoleBleue"),
        ("ÉcoleBleue", "ÉcoleBleue"),
        ("δ_value", "ΔValue"),
        ("ΔValue", "ΔValue"),
        ("StraßeÜber", "StraßeÜber"),
        ("中文组件", "中文组件"),
        ("e\u{301}_value", "E\u{301}Value"),
    ] {
        let ident = Ident::new(name, Span::call_site());
        let result = convert_from_snake_case(&ident).to_string();
        assert_eq!(
            result,
            previous_normalization(&ident).to_string(),
            "{name:?}"
        );
        assert_eq!(result, expected, "{name:?}");
    }
}

#[test]
fn raw_identifiers_preserve_previous_behavior() {
    for name in [
        "type",
        "async",
        "snake_case",
        "CustomElement",
        "HTTPServer",
        "école",
    ] {
        let ident = Ident::new_raw(name, Span::call_site());
        // Some raw identifiers are not valid after Pascal conversion. Preserve
        // the existing outcome rather than silently changing raw-name handling.
        let previous = std::panic::catch_unwind(|| previous_normalization(&ident).to_string());
        let current = std::panic::catch_unwind(|| convert_from_snake_case(&ident).to_string());
        match (previous, current) {
            (Ok(previous), Ok(current)) => assert_eq!(current, previous, "{name:?}"),
            (Err(_), Err(_)) => {}
            _ => panic!("raw identifier behavior changed: {name:?}"),
        }
    }
}
