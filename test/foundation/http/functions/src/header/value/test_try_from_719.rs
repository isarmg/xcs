#[test]
fn test_try_from() {
    HeaderValue::try_from(vec![127]).unwrap_err();
}
