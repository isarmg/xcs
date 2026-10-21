#[test]
fn test_array() {
    #[derive(Debug, PartialEq, data_serialization::Serialize, data_serialization::Deserialize)]
    struct Demo<'s>(#[serialization(borrow, deserialize_with = "array_of_cow")] [Cow<'s, str>; 1]);

    let data_orig = Demo(["Hello world".into()]);
    let json = json_codec::to_string(&data_orig).expect("serialize");
    let data_new = json_codec::from_str::<Demo>(&json).expect("deserialize");
    assert_eq!(data_orig, data_new);
    assert!(matches!(data_new.0, [Cow::Borrowed(_)]));
}
