use super::*;
use json_codec::json;
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn configuration_publication_is_private_complete_and_never_clobbers() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    let body = json!({"generation":u64::MAX.to_string()});
    save_json_private(&path, &body).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
        0o600
    );
    assert_eq!(
        json_codec::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
        body
    );
    assert!(save_json_private(&path, &json!({"changed":true})).is_err());
    assert_eq!(
        json_codec::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
        body
    );
}
