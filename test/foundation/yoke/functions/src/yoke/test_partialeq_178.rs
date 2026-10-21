#[test]
fn test_partialeq() {
    let a = Rc::new("same".to_string());
    let b = Rc::new("same".to_string());

    let y1 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(a);
    let y2 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(b);

    assert_eq!(y1, y2);
}
