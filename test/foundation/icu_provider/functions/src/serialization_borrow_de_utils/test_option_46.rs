#[test]
fn test_option() {
    #[derive(Debug, PartialEq, data_serialization::Serialize, data_serialization::Deserialize)]
    struct Demo<'s>(#[serialization(borrow, deserialize_with = "option_of_cow")] Option<Cow<'s, str>>);

    let data_orig = Demo(Some("Hello world".into()));
    let json = json_codec::to_string(&data_orig).expect("serialize");
    let data_new = json_codec::from_str::<Demo>(&json).expect("deserialize");
    assert_eq!(data_orig, data_new);
    assert!(matches!(data_new.0, Some(Cow::Borrowed(_))));
}
