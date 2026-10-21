#[test]
fn test_clone() {
    let local_data = "foo".to_owned();
    let y1 = Yoke::<alloc::borrow::Cow<'static, str>, Rc<String>>::attach_to_zero_copy_cart(
        Rc::new(local_data),
    );

    // Test basic clone
    let y2 = y1.clone();
    assert_eq!(y1.get(), "foo");
    assert_eq!(y2.get(), "foo");

    // Test clone with mutation on target
    let mut y3 = y1.clone();
    y3.with_mut(|y| {
        y.to_mut().push_str("bar");
    });
    assert_eq!(y1.get(), "foo");
    assert_eq!(y2.get(), "foo");
    assert_eq!(y3.get(), "foobar");

    // Test that mutations on source do not affect target
    let y4 = y3.clone();
    y3.with_mut(|y| {
        y.to_mut().push_str("baz");
    });
    assert_eq!(y1.get(), "foo");
    assert_eq!(y2.get(), "foo");
    assert_eq!(y3.get(), "foobarbaz");
    assert_eq!(y4.get(), "foobar");
}
