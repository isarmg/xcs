#[test]
fn match_serialize() {
    use serde::Serialize;
    #[derive(Serialize)]
    struct CustomStruct {
        field: &'static str,
    }

    let test = CustomStruct { field: "field" };
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": {"field":"field"}}"#);
    // Verification of ownership
    assert_eq!(test.field, "field");
}
