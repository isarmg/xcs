#[test]
fn find_map() {
    let mut it = convert(vec![0, 1, 2, 3].into_iter().map(Ok::<i32, ()>));
    assert_eq!(
        it.find_map(|v| match v {
            2 => Ok(Some("hi")),
            _ => Ok(None),
        }),
        Ok(Some("hi"))
    );
}
