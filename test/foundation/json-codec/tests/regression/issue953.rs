use json_codec::Value;

#[test]
fn test() {
    let x1 = json_codec::from_str::<Value>("18446744073709551615.");
    assert!(x1.is_err());
    let x2 = json_codec::from_str::<Value>("18446744073709551616.");
    assert!(x2.is_err());
}
