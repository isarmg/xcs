use super::*;
#[test]
fn missing_or_invalid_samples_are_never_reported_as_zero_usage_or_power() {
    assert_eq!(percent(&Value::Null), None);
    assert_eq!(ratio(&Value::Null, &Value::from(100)), None);
    assert_eq!(ratio(&Value::from(101), &Value::from(100)), None);
    assert_eq!(ratio(&Value::from(20), &Value::from(100)), Some(20.));
    assert_eq!(power(&Value::Null), None);
    assert_eq!(power(&Value::from(-1)), None);
    assert_eq!(
        Metric::CpuPower.value(&json_codec::json!({"cpu":{"power_watts":0.0}})),
        Some(0.)
    );
    assert_eq!(
        Metric::Disk {
            id: "P01".into(),
            name: "P01".into(),
            power: true
        }
        .value(&json_codec::json!({"disks":[{"id":"B01","power_watts":12.0}]})),
        None
    );
}
#[test]
fn changing_history_range_rejects_an_earlier_response_even_after_switching_back() {
    let mut model = Model::default();
    assert!(history_matches(&model, 0, "1min"));
    model.range = "24h".into();
    model.history_generation += 1;
    assert!(!history_matches(&model, 0, "1min"));
    model.range = "1min".into();
    model.history_generation += 1;
    assert!(!history_matches(&model, 0, "1min"));
    assert!(history_matches(&model, 2, "1min"));
}
