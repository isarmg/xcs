use super::*;
use std::{fs, os::unix::fs::PermissionsExt};
#[test]
fn password_requires_private_bounded_input() {
    let directory = tempfile::tempdir().unwrap();
    let password_path = directory.path().join("password");
    let maximum = "p".repeat(auth::PASSWORD_MAX_BYTES);
    private_io::write_new(&password_path, format!("{maximum}\r\n").as_bytes()).unwrap();
    assert_eq!(password(Some(&password_path)).unwrap(), maximum);
    fs::set_permissions(&password_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(password(Some(&password_path)).is_err());
}
