//! Bounded password/token input; secrets are never accepted as CLI arguments.
use crate::{Result, private_io};
use std::{io::Read, path::Path};
use zxs_auth as auth;

const PASSWORD_INPUT_BYTES: usize = auth::PASSWORD_MAX_BYTES + 2;
const TOKEN_INPUT_BYTES: usize = auth::SESSION_TOKEN_ENCODED_BYTES + 2;

pub(crate) fn password(file: Option<&Path>) -> Result<String> {
    let mut value = match file {
        Some(path) => private_io::read_private(path, PASSWORD_INPUT_BYTES)?,
        None => {
            eprintln!("Read password from standard input (minimum 12 bytes):");
            let mut value = String::new();
            std::io::stdin()
                .take(PASSWORD_INPUT_BYTES as u64 + 1)
                .read_to_string(&mut value)?;
            if value.len() > PASSWORD_INPUT_BYTES {
                return Err("password input exceeds its byte limit".into());
            }
            value
        }
    };
    while value.ends_with('\n') || value.ends_with('\r') {
        value.pop();
    }
    auth::validate_password(&value)?;
    Ok(value)
}

pub(crate) fn save_token(path: &Path, token: &str) -> Result<()> {
    if !auth::is_token_shape(token) {
        return Err("invalid session token".into());
    }
    private_io::write_new(path, token.as_bytes())
}
pub(crate) fn load_token(path: &Path) -> Result<String> {
    let token = private_io::read_private(path, TOKEN_INPUT_BYTES)?
        .trim()
        .to_owned();
    if !auth::is_token_shape(&token) {
        return Err("invalid session token file".into());
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
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
}
