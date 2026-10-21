//! Request-scoped public projections. Authorization and version selection happen
//! against the same locked engine; only owned response data crosses the boundary.
//! This is not a cache or a reusable authorization grant, and carries no State,
//! private authentication data, content read plan, or publication capability.
use super::{authorize, encode, error, field, pending_entry_source, repo_for_branch};
use crate::{ApiError, auth::Identity};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, sync::MutexGuard};
use storage_engine::Engine;

#[derive(Serialize)]
pub(super) struct Tree {
    branch_id: String,
    commit_id: String,
    entries: Vec<storage_contract::Entry>,
    pending_file_versions: BTreeMap<String, String>,
}

impl Tree {
    pub(super) fn capture(
        engine: &Engine,
        identity: &Identity,
        input: &Value,
    ) -> Result<Self, ApiError> {
        let branch = field(input, "branch_id")?;
        authorize(identity, repo_for_branch(engine, branch)?, false, false)?;
        let branch_info = engine.state().branches.get(branch).unwrap();
        let commit = input
            .get("query")
            .and_then(|v| v.get("commit_id"))
            .and_then(Value::as_str)
            .unwrap_or(&branch_info.head_commit_id);
        let snapshot = engine
            .state()
            .commits
            .get(commit)
            .filter(|commit| commit.history_tree_id == branch_info.history_tree_id)
            .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "commit not found"))?;
        let entries = engine.list_tree_at(branch, commit).map_err(error)?;
        let pending_file_versions = entries
            .iter()
            .filter_map(|entry| {
                pending_entry_source(engine, branch, snapshot, entry)
                    .map(|source| (entry.file_id.clone(), source.to_owned()))
            })
            .collect();
        Ok(Self {
            branch_id: branch.to_owned(),
            commit_id: commit.to_owned(),
            entries,
            pending_file_versions,
        })
    }
}

// Taking ownership of the guard makes the release point explicit. Projections
// must own their data: borrowing from this guard cannot survive its drop.
pub(super) fn render_unlocked<T: Serialize>(
    engine: MutexGuard<'_, Engine>,
    projection: T,
) -> Result<Value, ApiError> {
    drop(engine);
    encode(projection)
}

#[cfg(test)]
#[path = "../../../../test/server/unit/engine_backend/read_projection/tests.rs"]
mod tests;
