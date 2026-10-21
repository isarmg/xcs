#[test]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
fn default_roundtrip() {
    DocumentMut::default()
        .to_string()
        .parse::<DocumentMut>()
        .unwrap();
}
