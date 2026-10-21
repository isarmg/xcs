use super::*;

#[test]
fn namespace_record_noclobber_preserves_existing_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operation.namespace.json");
    fs::write(&path, b"earlier recovery evidence").unwrap();
    assert!(write_namespace_record(&path, &json!({"replacement":true})).is_err());
    assert_eq!(fs::read(path).unwrap(), b"earlier recovery evidence");
}
