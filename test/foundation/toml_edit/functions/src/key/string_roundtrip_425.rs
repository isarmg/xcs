#[test]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
fn string_roundtrip() {
    Key::new("hello").to_string().parse::<Key>().unwrap();
}
