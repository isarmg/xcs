#![cfg(feature = "arbitrary_precision")]

#[test]
fn test() {
    let float = 5.55f32;
    let value = json_codec::to_value(float).unwrap();
    let json = json_codec::to_string(&value).unwrap();

    // If the f32 were cast to f64 by Value before serialization, then this
    // would incorrectly serialize as 5.550000190734863.
    assert_eq!(json, "5.55");
}
