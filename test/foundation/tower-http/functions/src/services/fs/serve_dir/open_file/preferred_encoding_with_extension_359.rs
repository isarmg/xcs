#[test]
fn preferred_encoding_with_extension() {
    let mut path = PathBuf::from("hello.txt");
    preferred_encoding(&mut path, &[(Encoding::Gzip, QValue::one())]);
    assert_eq!(path, PathBuf::from("hello.txt.gz"));
}
