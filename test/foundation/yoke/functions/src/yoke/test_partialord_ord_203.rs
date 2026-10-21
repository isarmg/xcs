#[test]
fn test_partialord_ord() {
    let a = Rc::new("a".to_string());
    let b = Rc::new("b".to_string());

    let y1 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(a);
    let y2 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(b);

    assert!(y1 < y2);
    assert_eq!(y1.partial_cmp(&y2), Some(core::cmp::Ordering::Less));
}
