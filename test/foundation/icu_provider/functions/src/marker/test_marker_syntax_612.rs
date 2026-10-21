#[test]
fn test_marker_syntax() {
    // Valid markers:
    DataMarkerId::from_name("HelloWorldV1").unwrap();
    DataMarkerId::from_name("HelloWorldFooV1").unwrap();
    DataMarkerId::from_name("HelloWorldV999").unwrap();
    DataMarkerId::from_name("Hello485FooV1").unwrap();

    // No version:
    assert_eq!(
        DataMarkerId::from_name("HelloWorld"),
        Err(("[0-9]", "HelloWorld".len()))
    );

    assert_eq!(
        DataMarkerId::from_name("HelloWorldV"),
        Err(("[0-9]", "HelloWorldV".len()))
    );
    assert_eq!(
        DataMarkerId::from_name("HelloWorldVFoo"),
        Err(("[0-9]", "HelloWorldVFoo".len()))
    );
    assert_eq!(
        DataMarkerId::from_name("HelloWorldV1Foo"),
        Err(("[0-9]", "HelloWorldV1Foo".len()))
    );
}
