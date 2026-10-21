#[test]
fn skip_duplicates_during_key_iteration() {
    let mut map = HeaderMap::new();
    map.try_append("a", HeaderValue::from_static("a")).unwrap();
    map.try_append("a", HeaderValue::from_static("b")).unwrap();
    assert_eq!(map.keys().count(), map.keys_len());
}
