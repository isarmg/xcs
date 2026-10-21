#[test]
fn test_display() {
    let local_data = "hello".to_owned();
    let y = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(
        Rc::new(local_data),
    );
    assert_eq!(format!("{}", y), "hello");
}
