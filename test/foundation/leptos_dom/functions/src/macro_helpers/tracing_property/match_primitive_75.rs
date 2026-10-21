#[test]
fn match_primitive() {
    // String
    let test = String::from("string");
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": "string"}"#);

    // &str
    let test = "string";
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": "string"}"#);

    // u128
    let test: u128 = 1;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": 1}"#);

    // i128
    let test: i128 = -1;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": -1}"#);

    // f64
    let test = 3.25;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": 3.25}"#);

    // bool
    let test = true;
    let prop = (&&Match {
        name: stringify! {test},
        value: std::cell::Cell::new(Some(&test)),
    })
        .spez();
    assert_eq!(prop, r#"{"name": "test", "value": true}"#);
}
