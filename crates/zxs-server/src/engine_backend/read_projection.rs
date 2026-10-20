//! Request-scoped public projections. Authorization and version selection happen
//! against the same locked engine; only owned response data crosses the boundary.
//! This is not a cache or a reusable authorization grant, and carries no State,
//! private authentication data, content read plan, or publication capability.
use super::{authorize, encode, error, field, pending_entry_source, repo_for_branch};
use crate::{ApiError, auth::Identity};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, sync::MutexGuard};
use zxs_core::Engine;

#[derive(Serialize)]
pub(super) struct Tree {
    branch_id: String,
    commit_id: String,
    entries: Vec<zxs_contract::Entry>,
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
mod tests {
    use super::*;
    use crate::{Backend, BackendAction, engine_backend::EngineBackend};
    use serde::{Serializer, ser::Error};
    use serde_json::json;
    use std::{collections::BTreeSet, sync::Mutex};
    use zxs_contract::{Config, new_id};

    fn fixture() -> (tempfile::TempDir, Engine, zxs_core::Repository, Identity) {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let repo = engine
            .create_repository("Projection", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        let identity = Identity {
            principal: "owner".into(),
            admin: false,
            repositories: BTreeSet::from([repo.repository_id.clone()]),
        };
        (root, engine, repo, identity)
    }

    #[test]
    fn rendering_releases_engine_before_serializer_runs() {
        let (_root, engine, _, _) = fixture();
        let engine = Mutex::new(engine);
        struct Probe<'a>(&'a Mutex<Engine>);
        impl Serialize for Probe<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                // Runs on another thread to prove this is not reentrant access.
                let available = std::thread::scope(|scope| {
                    scope.spawn(|| self.0.try_lock().is_ok()).join().unwrap()
                });
                if !available {
                    return Err(S::Error::custom("serialization retained Engine mutex"));
                }
                serializer.serialize_bool(available)
            }
        }
        assert_eq!(
            render_unlocked(engine.lock().unwrap(), Probe(&engine)).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn tree_projection_is_frozen_and_preserves_authorization_and_wire_shape() {
        let (_root, mut engine, repo, identity) = fixture();
        let input = json!({"branch_id": repo.main_branch_id});
        let expected = json!({
            "branch_id": repo.main_branch_id,
            "commit_id": engine.state().branches[&repo.main_branch_id].head_commit_id,
            "entries": engine.list_tree(&repo.main_branch_id).unwrap(),
            "pending_file_versions": {},
        });
        let projection = Tree::capture(&engine, &identity, &input).unwrap();
        let mut denied = identity.clone();
        denied.repositories.clear();
        assert_eq!(
            Tree::capture(&engine, &denied, &input).err().unwrap().code,
            "FORBIDDEN"
        );
        let other = engine
            .create_repository("Other", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        let foreign = &engine.state().branches[&other.main_branch_id].head_commit_id;
        assert_eq!(
            Tree::capture(
                &engine,
                &identity,
                &json!({
                    "branch_id": repo.main_branch_id, "query": {"commit_id": foreign}
                })
            )
            .err()
            .unwrap()
            .code,
            "NOT_FOUND"
        );
        // The already admitted response owns its snapshot even after live changes.
        assert_eq!(encode(projection).unwrap(), expected);
        let backend = EngineBackend::new(engine);
        assert_eq!(
            backend
                .call(BackendAction::Tree, &identity, input.clone())
                .unwrap(),
            expected
        );
        assert_eq!(
            backend
                .call(BackendAction::Tree, &denied, input.clone())
                .unwrap_err()
                .code,
            "FORBIDDEN"
        );
        let frozen = {
            let mut engine = backend.engine.lock().unwrap();
            let frozen = Tree::capture(&engine, &identity, &input).unwrap();
            engine
                .set_grants(
                    &repo.repository_id,
                    zxs_core::Grants {
                        manage: BTreeSet::from(["replacement-owner".into()]),
                        ..Default::default()
                    },
                    &identity.principal,
                    &new_id("op"),
                )
                .unwrap();
            frozen
        };
        assert_eq!(encode(frozen).unwrap(), expected);
        // Frozen response data never grants admission to a later request.
        assert_eq!(
            backend
                .call(BackendAction::Tree, &identity, input)
                .unwrap_err()
                .code,
            "FORBIDDEN"
        );
    }
}
