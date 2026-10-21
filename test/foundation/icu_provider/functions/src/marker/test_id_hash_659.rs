#[test]
fn test_id_hash() {
    assert_eq!(
        DataMarkerId::from_name("BarV1").unwrap().hashed(),
        DataMarkerIdHash([212, 77, 158, 241]),
    );
}
