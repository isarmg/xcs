#[test]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
fn string_roundtrip() {
    Value::from("hello").to_string().parse::<Value>().unwrap();
}
