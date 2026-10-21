#[test]
fn parse_key_value_with_comment_test() {
    let ini_file = &b"parameter=value;abc
key = value2"[..];

    let ini_without_key_value = &b"\nkey = value2"[..];

    let res = key_value.parse_peek(ini_file);
    println!("{res:?}");
    match res {
        Ok((i, (o1, o2))) => println!("i: {:?} | o: ({:?},{:?})", str::from_utf8(i), o1, o2),
        _ => println!("error"),
    }

    assert_eq!(res, Ok((ini_without_key_value, ("parameter", "value"))));
}
