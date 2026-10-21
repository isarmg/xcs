//! Bounded password/token input; secrets are never accepted as CLI arguments.
use crate::{Result, private_io};
use authentication as auth;
use std::{io::Read, path::Path};

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
#[path = "../../../test/admin/unit/credentials/tests.rs"]
mod tests;
