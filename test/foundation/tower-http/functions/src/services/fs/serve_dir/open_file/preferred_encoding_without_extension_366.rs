#[test]
fn preferred_encoding_without_extension() {
    let mut path = PathBuf::from("hello");
    preferred_encoding(&mut path, &[(Encoding::Gzip, QValue::one())]);
    assert_eq!(path, PathBuf::from("hello.gz"));
}
