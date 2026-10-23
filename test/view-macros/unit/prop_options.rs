use super::*;

type Expected = std::result::Result<&'static str, &'static [&'static str]>;

#[path = "prop_options_cases.rs"]
mod cases;

fn attributes(source: &str) -> Vec<Attribute> {
    Attribute::parse_outer.parse_str(source).unwrap()
}

fn snapshot<const COMPONENT: bool>(options: PropOptions<COMPONENT>) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{:?}|{}",
        options.optional,
        options.optional_no_strip,
        options.strip_option,
        options
            .default
            .map(|expr| expr.to_token_stream().to_string())
            .unwrap_or_default(),
        options.into,
        options.attrs,
        options.name,
        options.marker
    )
}

fn diagnostic(error: Error) -> Vec<String> {
    error.into_iter().map(|error| error.to_string()).collect()
}

// Captured from the previous parser on 794c3514. The generic derive dependency
// is deliberately absent: tests must not retain the retired build-time closure.
#[test]
fn accepted_values_and_diagnostics_match_the_reference_grammar() {
    assert_eq!(cases::CASES.len(), 269);
    for &(source, component, slot) in cases::CASES {
        let attrs = attributes(source);
        let owned = |expected: Expected| {
            expected.map(str::to_owned).map_err(|errors| {
                errors
                    .iter()
                    .map(|error| (*error).to_owned())
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(
            PropOptions::<true>::from_attributes(&attrs)
                .map(snapshot)
                .map_err(diagnostic),
            owned(component),
            "component: {source}"
        );
        assert_eq!(
            PropOptions::<false>::from_attributes(&attrs)
                .map(snapshot)
                .map_err(diagnostic),
            owned(slot),
            "slot: {source}"
        );
    }
}

#[test]
fn errors_point_to_the_relevant_input() {
    for (source, expected) in [
        ("#[prop(unknown)]", "unknown"),
        (
            "#[prop(optional = false, optional = true)]",
            "false, optional = true",
        ),
        (
            "#[prop(optional(false), optional(true))]",
            "false), optional(true",
        ),
        ("#[prop(optional, strip_option)]", "optional, strip_option"),
        ("#[prop(default = 1, default = 2)]", "1, default = 2"),
    ] {
        let error = PropOptions::<true>::from_attributes(&attributes(source)).unwrap_err();
        assert_eq!(
            error.span().source_text().as_deref(),
            Some(expected),
            "{source}"
        );
    }
}
