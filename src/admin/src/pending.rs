//! Complete no-clobber publication of configuration files.
use crate::{Result, private_io};
use json_codec::Value;
use std::path::Path;

pub(crate) fn save_json_private(path: &Path, value: &Value) -> Result<()> {
    private_io::write_new(path, &json_codec::to_vec_pretty(value)?)
}

#[cfg(test)]
#[path = "../../../test/admin/unit/pending/tests.rs"]
mod tests;
