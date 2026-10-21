#[test]
fn simple() {
    let data = "\"abc\"";
    let result = parser::parse_string::<()>.parse(data);
    assert_eq!(result, Ok(String::from("abc")));
}
