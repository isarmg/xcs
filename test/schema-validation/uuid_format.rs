use json_codec::json;
use schema_validation::Draft;

#[test]
fn uuid_format_matches_exact_syntax_in_both_supported_drafts() {
    for draft in [Draft::Draft201909, Draft::Draft202012] {
        let validator = schema_validation::options()
            .with_draft(draft)
            .should_validate_formats(true)
            .build(&json!({"type":"string", "format":"uuid"}))
            .unwrap();
        for text in [
            // Retained upstream UUID unit and WASM cases, plus wider boundaries.
            "f308a72c-fa84-11eb-9a03-0242ac130003",
            "67e55044-10b1-426f-9247-bb680e5fe0c8",
            "67E55044-10B1-426F-9247-BB680E5FE0C8",
            "00000000-0000-0000-0000-000000000000",
            "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
            "12345678-9aBc-DeF0-1234-56789abcdef0",
        ] {
            assert!(validator.is_valid(&json!(text)), "{text}");
            assert!(validator.validate(&json!(text)).is_ok());
            for i in 0..36 {
                let mut bytes = text.as_bytes().to_vec();
                bytes[i] = b'!';
                let invalid = String::from_utf8(bytes).unwrap();
                assert!(!validator.is_valid(&json!(invalid)));
            }
        }
        for text in [
            "",
            "1",
            "67e5504410b1426f9247bb680e5fe0c8",
            "{67e55044-10b1-426f-9247-bb680e5fe0c8}",
            "urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8",
            "12345678-9abc-def0-1234-56789abcdeé",
        ] {
            assert!(!validator.is_valid(&json!(text)), "{text}");
        }
    }
}

/// Host-only validation micro-workload, not a product hot-path benchmark.
/// Run the exact same test against both source revisions before comparing.
#[test]
#[ignore]
fn uuid_validation_throughput() {
    use std::{hint::black_box, time::Instant};
    let validator = schema_validation::options()
        .with_draft(Draft::Draft202012)
        .should_validate_formats(true)
        .build(&json!({"type":"object", "properties":{"id":{"type":"string", "format":"uuid"}}, "required":["id"], "additionalProperties":false})).unwrap();
    let values = [
        json!({"id":"12345678-9aBc-DeF0-1234-56789abcdef0"}),
        json!({"id":"00000000-0000-0000-0000-000000000000"}),
        json!({"id":"FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"}),
        json!({"id":"!2345678-9aBc-DeF0-1234-56789abcdef0"}),
        json!({"id":"12345678-9aBc-DeF0-1234-56789abcdef!"}),
        json!({"id":"12345678-9aBc-DeF0_1234-56789abcdef0"}),
        json!({"id":"12345678-9aBc-DeF0-1234-56789abcdef"}),
    ];
    let mut ns = Vec::new();
    for _ in 0..11 {
        let start = Instant::now();
        let mut valid = 0;
        for i in 0..1_000_000 {
            valid += validator.is_valid(black_box(&values[i % values.len()])) as usize;
        }
        black_box(valid);
        ns.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
    }
    ns.sort_by(f64::total_cmp);
    println!(
        "mixed object UUID validation: min {:.3}, median {:.3}, p90 {:.3} ns/instance",
        ns[0], ns[5], ns[9]
    );
}
