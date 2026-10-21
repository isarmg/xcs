use super::*;
#[test]
fn integer_parsing_retains_large_disk_capacities_without_javascript_rounding() {
    assert_eq!(
        parse_u64(&Value::String("18446744073709551615".into())),
        Some(u64::MAX)
    );
    for invalid in ["001", "-1", "1.5", ""] {
        assert_eq!(parse_u64(&Value::String(invalid.into())), None);
    }
    let record = disk_record(
        serde_json::json!({"total_bytes":"18446744073709551615","available_bytes":"1"}),
    );
    assert_eq!(record["used_bytes"], "18446744073709551614");
}
#[test]
fn human_sizes_use_binary_units_in_both_languages() {
    assert_eq!(bytes(1024), "1.0 KiB");
    assert_eq!(bytes(42), "42 B");
    assert!(bytes(u64::MAX).ends_with("EiB"));
}
