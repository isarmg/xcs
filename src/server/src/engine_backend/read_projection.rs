//! Request-scoped public projections. Authorization and version selection happen
//! against the same locked engine; only owned response data crosses the boundary.
//! This is not a cache or a reusable authorization grant, and carries no State,
//! private authentication data, content read plan, or publication capability.
use super::wire::PendingSources;
use super::{authorize, encode, error, field, repo_for_branch};
use crate::{ApiError, auth::Identity};
use data_serialization::Serialize;
use json_codec::Value;
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
        let pending_file_versions =
            PendingSources::new(engine, branch, snapshot).pending_versions();
        Ok(Self {
            branch_id: branch.to_owned(),
            commit_id: commit.to_owned(),
            entries,
            pending_file_versions,
        })
    }
}

/// A bounded projection over the immutable, lookup-key-sorted commit tree.
/// It neither clones nor serializes the complete namespace for every page.
#[derive(Serialize)]
pub(super) struct SmbDirectory {
    commit_id: String,
    entries: Vec<storage_contract::Entry>,
    more: bool,
}
impl SmbDirectory {
    pub(super) fn capture(
        engine: &Engine,
        identity: &Identity,
        input: &Value,
    ) -> Result<Self, ApiError> {
        let branch = field(input, "branch_id")?;
        authorize(identity, repo_for_branch(engine, branch)?, false, false)?;
        let branch_info = engine.state().branches.get(branch).unwrap();
        let commit = input
            .get("commit_id")
            .and_then(Value::as_str)
            .unwrap_or(&branch_info.head_commit_id);
        let snapshot = engine
            .state()
            .commits
            .get(commit)
            .filter(|snapshot| snapshot.history_tree_id == branch_info.history_tree_id)
            .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "directory snapshot missing"))?;
        // The synthetic share root has an intentionally empty path.
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::invalid("directory path must be a string"))?;
        let key = name_policy::path_key(path);
        let entry_id = field(input, "entry_id")?;
        if path.is_empty() {
            if entry_id != branch {
                return Err(ApiError::invalid("invalid root directory identity"));
            }
        } else {
            name_policy::validate_path(path).map_err(ApiError::invalid)?;
            let directory = snapshot
                .entries
                .binary_search_by(|entry| entry.lookup_key.cmp(&key))
                .ok()
                .and_then(|index| snapshot.entries.get(index));
            if directory.is_none_or(|entry| entry.kind != "directory" || entry.entry_id != entry_id)
            {
                return Err(ApiError::new(
                    409,
                    "SMB_REBIND_REQUIRED",
                    "opened directory identity changed",
                ));
            }
        }
        if input.get("capture_only").and_then(Value::as_bool) == Some(true) {
            return Ok(Self {
                commit_id: snapshot.commit_id.clone(),
                entries: Vec::new(),
                more: false,
            });
        }
        Self::page(
            &snapshot.commit_id,
            &snapshot.entries,
            &key,
            input.get("after").and_then(Value::as_str),
        )
    }

    fn page(
        commit_id: &str,
        entries: &[storage_contract::Entry],
        key: &str,
        after: Option<&str>,
    ) -> Result<Self, ApiError> {
        use smb_protocol::protocol::{
            MAX_DIRECTORY_PAGE_BYTES, MAX_DIRECTORY_PAGE_ENTRIES, Response,
        };
        let prefix = if key.is_empty() {
            String::new()
        } else {
            format!("{key}/")
        };
        let start = entries.partition_point(|entry| {
            entry.lookup_key.as_str() < prefix.as_str()
                || after.is_some_and(|after| entry.lookup_key.as_str() <= after)
        });
        let mut page = Self {
            commit_id: commit_id.into(),
            entries: Vec::new(),
            more: false,
        };
        // Reserve the larger internal response envelope. The wire strips the
        // commit id; false->true only reduces the encoded length.
        let mut bytes = json_codec::to_vec(&Response::Ok {
            result: json_codec::to_value(&page).map_err(ApiError::invalid)?,
        })
        .map_err(ApiError::invalid)?
        .len();
        for entry in &entries[start..] {
            let Some(rest) = entry.lookup_key.strip_prefix(&prefix) else {
                break;
            };
            if rest.is_empty() || rest.contains('/') {
                continue;
            }
            let extra = json_codec::to_vec(entry).map_err(ApiError::invalid)?.len()
                + usize::from(!page.entries.is_empty());
            if page.entries.len() >= MAX_DIRECTORY_PAGE_ENTRIES
                || bytes.saturating_add(extra) > MAX_DIRECTORY_PAGE_BYTES
            {
                if page.entries.is_empty() {
                    return Err(ApiError::limit("directory entry exceeds page byte budget"));
                }
                page.more = true;
                break;
            }
            bytes += extra;
            page.entries.push(entry.clone());
        }
        Ok(page)
    }
}

// Taking ownership of the guard makes the release point explicit. Projections
// must own their data: borrowing from this guard cannot survive its drop.
pub(super) fn render_unlocked<T: Serialize, E>(
    engine: MutexGuard<'_, E>,
    projection: T,
) -> Result<Value, ApiError> {
    drop(engine);
    encode(projection)
}

#[cfg(test)]
#[path = "../../../../test/server/unit/engine_backend/read_projection/tests.rs"]
mod tests;
