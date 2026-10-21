use super::*;
use std::{fs, os::unix::fs::PermissionsExt};
#[test]
fn token_and_password_use_the_same_private_file_admission() {
    let directory = tempfile::tempdir().unwrap();
    let token = auth::random_token().unwrap();
    let token_path = directory.path().join("token");
    save_token(&token_path, &token).unwrap();
    assert_eq!(load_token(&token_path).unwrap(), token);
    assert!(save_token(&token_path, &token).is_err());
    let password_path = directory.path().join("password");
    let maximum = "p".repeat(auth::PASSWORD_MAX_BYTES);
    private_io::write_new(&password_path, format!("{maximum}\r\n").as_bytes()).unwrap();
    assert_eq!(password(Some(&password_path)).unwrap(), maximum);
    fs::set_permissions(&password_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(password(Some(&password_path)).is_err());
    fs::set_permissions(&token_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load_token(&token_path).is_err());
}
