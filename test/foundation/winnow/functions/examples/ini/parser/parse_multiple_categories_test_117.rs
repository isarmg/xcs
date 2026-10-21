#[test]
fn parse_multiple_categories_test() {
    let ini_file = &b"[abcd]

parameter=value;abc

key = value2

[category]
parameter3=value3
key4 = value4
"[..];

    let ini_after_parser = &b""[..];

    let res = categories.parse_peek(ini_file);
    //println!("{:?}", res);
    match res {
        Ok((i, ref o)) => println!("i: {:?} | o: {:?}", str::from_utf8(i), o),
        _ => println!("error"),
    }

    let mut expected_1: HashMap<&str, &str> = HashMap::new();
    expected_1.insert("parameter", "value");
    expected_1.insert("key", "value2");
    let mut expected_2: HashMap<&str, &str> = HashMap::new();
    expected_2.insert("parameter3", "value3");
    expected_2.insert("key4", "value4");
    let mut expected_h: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    expected_h.insert("abcd", expected_1);
    expected_h.insert("category", expected_2);
    assert_eq!(res, Ok((ini_after_parser, expected_h)));
}
