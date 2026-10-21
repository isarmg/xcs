#[test]
fn test_id_debug() {
    assert_eq!(DataMarkerId::from_name("BarV1").unwrap().debug, "BarV1");
}
