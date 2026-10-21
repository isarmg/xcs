#[test]
fn test_eq_trait() {
    let x = Rc::new("equal".to_string());
    let y = Rc::new("equal".to_string());

    let y1 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(x);
    let y2 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(y);

    assert!(y1 == y2);

    let vec = [y1];
    assert!(vec.contains(&y2));
}
