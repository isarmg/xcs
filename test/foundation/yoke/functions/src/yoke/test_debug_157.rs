#[test]
fn test_debug() {
    let local_data = "foo".to_owned();
    let y1 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(
        Rc::new(local_data),
    );
    assert_eq!(
        format!("{y1:?}"),
        r#"Yoke { yokeable: "foo", cart: "foo" }"#,
    );
}
