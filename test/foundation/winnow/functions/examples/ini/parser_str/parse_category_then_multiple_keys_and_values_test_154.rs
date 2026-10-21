#[test]
fn parse_category_then_multiple_keys_and_values_test() {
    //FIXME: there can be an empty line or a comment line after a category
    let ini_file = "[abcd]
parameter=value;abc

key = value2

[category]";

    let ini_after_parser = "[category]";

    let res = category_and_keys.parse_peek(ini_file);
    println!("{res:?}");
    match res {
        Ok((i, ref o)) => println!("i: {i} | o: {o:?}"),
        _ => println!("error"),
    }

    let mut expected_h: HashMap<&str, &str> = HashMap::new();
    expected_h.insert("parameter", "value");
    expected_h.insert("key", "value2");
    assert_eq!(res, Ok((ini_after_parser, ("abcd", expected_h))));
}
