#[test]
fn parse_multiple_keys_and_values_test() {
    let ini_file = "parameter=value;abc

key = value2

[category]";

    let ini_without_key_value = "[category]";

    let res = keys_and_values.parse_peek(ini_file);
    println!("{res:?}");
    match res {
        Ok((i, ref o)) => println!("i: {i} | o: {o:?}"),
        _ => println!("error"),
    }

    let mut expected: HashMap<&str, &str> = HashMap::new();
    expected.insert("parameter", "value");
    expected.insert("key", "value2");
    assert_eq!(res, Ok((ini_without_key_value, expected)));
}
