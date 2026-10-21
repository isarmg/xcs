#[test]
#[allow(clippy::needless_borrow)]
fn match_no_serialize() {
    struct CustomStruct {
        field: &'static str,
    }

    let test = CustomStruct { field: "field" };
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(
        prop,
        r#"{"name": "test", "value": "[unserializable value]"}"#
    );
    // Verification of ownership
    assert_eq!(test.field, "field");
}
