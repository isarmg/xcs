#[test]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
fn string_roundtrip() {
    value("hello").to_string().parse::<Item>().unwrap();
}
