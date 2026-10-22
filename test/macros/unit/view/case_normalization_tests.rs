use super::*;

// This is the previous implementation, including the extras predicate's
// exact definition. Keep the oracle independent of the optimized helper.
fn previous_normalization(name: String) -> String {
    if name != name.to_case(Snake) {
        name.to_case(Snake)
    } else {
        name
    }
}

#[test]
fn snake_case_preserves_previous_output() {
    for name in [
        "",
        "snake_case",
        "lowercase",
        "CamelCase",
        "HTTPServer",
        "XMLHttpRequest",
        "version2Value",
        "version_2_value",
        "99Bottles",
        "_private",
        "two__words",
        "custom-element",
        "custom:event",
        "r#type",
        "r#CustomElement",
        "ÉcoleBleue",
        "école_bleue",
        "ΔValue",
        "δ_value",
        "StraßeÜber",
        "İstanbul",
        "中文组件",
        "e\u{301}Value",
        "👩‍💻Widget",
    ] {
        assert_eq!(
            convert_to_snake_case(name.into()),
            previous_normalization(name.into()),
            "{name:?}"
        );
    }
}

#[test]
fn snake_case_matches_previous_output_across_boundary_combinations() {
    let parts = [
        "a", "B", "HTTP", "2", "_", "-", ":", "é", "Δ", "中", "\u{301}",
    ];
    for first in parts {
        for second in parts {
            for third in parts {
                let name = format!("{first}{second}{third}");
                assert_eq!(
                    convert_to_snake_case(name.clone()),
                    previous_normalization(name.clone()),
                    "{name:?}"
                );
            }
        }
    }
}
