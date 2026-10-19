use serde_json::Value;
use xcs_web::i18n::t;

pub fn bytes(value: u64) -> String {
    if value < 1024 {
        return format!("{value} B");
    }
    let mut n = value as f64;
    let mut index: usize = 0;
    let units = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    while n >= 1024. && index < units.len() {
        n /= 1024.;
        index += 1;
    }
    format!("{n:.1} {}", units[index.saturating_sub(1)])
}
pub fn parse_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => {
            if s == "0" || !s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit()) {
                s.parse().ok()
            } else {
                None
            }
        }
        _ => None,
    }
}
pub fn disk_record(mut v: Value) -> Value {
    if let (Some(total), Some(available)) = (
        parse_u64(&v["total_bytes"]),
        parse_u64(&v["available_bytes"]),
    ) && total > 0
        && available <= total
    {
        let used = total - available;
        v["used_bytes"] = if used > 9_007_199_254_740_991 {
            Value::String(used.to_string())
        } else {
            Value::from(used)
        };
        v["usage_percent"] = Value::from(used as f64 * 100. / total as f64);
    }
    v
}
pub fn timestamp(v: &str) -> String {
    if v == "—" || v.is_empty() {
        return t("尚无记录", "No record yet");
    }
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(v));
    if date.get_time().is_nan() {
        return t("未知", "Unknown");
    }
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"timeZone".into(), &"UTC".into());
    let _ = js_sys::Reflect::set(&opts, &"hourCycle".into(), &"h23".into());
    format!(
        "{} UTC",
        date.to_locale_string(&xcs_web::i18n::locale(), &opts)
            .as_string()
            .unwrap_or_default()
    )
}

/// File history timestamps are stored in seconds rather than browser milliseconds.
pub fn timestamp_seconds(value: &str) -> String {
    let Ok(seconds) = value.parse::<u64>() else {
        return t("未记录时间", "Time not recorded");
    };
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(seconds as f64 * 1000.));
    if date.get_time().is_nan() {
        return t("未记录时间", "Time not recorded");
    }
    timestamp(&date.to_iso_string().as_string().unwrap_or_default())
}
#[cfg(test)]
mod tests {
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
}
