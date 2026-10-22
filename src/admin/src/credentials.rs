//! Bounded password input; secrets are never accepted as CLI arguments.
use crate::{Result, private_io};
use authentication as auth;
use std::{io::Read, path::Path};

const PASSWORD_INPUT_BYTES: usize = auth::PASSWORD_MAX_BYTES + 2;

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

#[cfg(test)]
#[path = "../../../test/admin/unit/credentials/tests.rs"]
mod tests;
