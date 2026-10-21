#[test]
fn parse_category_test() {
    let ini_file = "[category]

parameter=value
key = value2";

    let ini_without_category = "parameter=value
key = value2";

    let res = category.parse_peek(ini_file);
    println!("{res:?}");
    match res {
        Ok((i, o)) => println!("i: {i} | o: {o:?}"),
        _ => println!("error"),
    }

    assert_eq!(res, Ok((ini_without_category, "category")));
}
