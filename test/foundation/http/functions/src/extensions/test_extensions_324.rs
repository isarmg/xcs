#[test]
fn test_extensions() {
    #[derive(Clone, Debug, PartialEq)]
    struct MyType(i32);

    let mut extensions = Extensions::new();
    assert_eq!(format!("{extensions:?}"), "{}");

    extensions.insert(5i32);
    extensions.insert(MyType(10));

    assert_eq!(extensions.get(), Some(&5i32));
    assert_eq!(extensions.get_mut(), Some(&mut 5i32));

    let dbg = format!("{extensions:?}");
    // map order is NOT deterministic
    assert!(
        (dbg == "{http::extensions::test_extensions::MyType, i32}")
            || (dbg == "{i32, http::extensions::test_extensions::MyType}"),
        "{}",
        dbg
    );

    let ext2 = extensions.clone();

    assert_eq!(extensions.remove::<i32>(), Some(5i32));
    assert!(extensions.get::<i32>().is_none());

    // clone still has it
    assert_eq!(ext2.get(), Some(&5i32));
    assert_eq!(ext2.get(), Some(&MyType(10)));

    assert_eq!(extensions.get::<bool>(), None);
    assert_eq!(extensions.get(), Some(&MyType(10)));
}
