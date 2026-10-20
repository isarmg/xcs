//! Shared namespace rules and domain operations.
use crate::*;
use std::time::{SystemTime, UNIX_EPOCH};
mod ancestry;
mod batch;
mod directory;
mod file;
pub(crate) mod file_merges;
mod file_metadata;
pub(crate) mod file_nodes;
mod file_paths;
pub(crate) mod file_publication;
mod file_tags;
mod history;
mod namespace;
mod repository;
mod workspace;
pub(crate) use ancestry::descendant_commits;
pub(crate) use namespace::{
    apply_namespace, apply_workspace_namespace, ensure_parents, find, sorted, validate_entries,
    validate_workspace_dependencies,
};
pub use workspace::{PreparedUpload, UploadPreparation};

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn validate_branch_name(name: &str) -> Result<()> {
    if name.len() > 128 {
        return Err(Error::new(
            "METADATA_TOO_LONG",
            "branch names must not exceed 128 UTF-8 bytes",
        ));
    }
    xcs_naming::validate_name(name)
}

#[cfg(test)]
mod composition_tests;

#[cfg(test)]
mod file_paths_tests;
