//! Typed, transport-independent mutation boundary.
//!
//! HTTP and SMB decode their envelopes before entering this service. Account
//! admission, repository ACLs and workspace ownership are checked here, before
//! delegating durable state changes to the core. Results remain domain records;
//! presentation/redaction belongs to the transport adapter.
use super::{
    RepositoryAccess, authorize_access, error, repo_for_branch, repo_for_history, repository,
    workspace_authorize,
};
use crate::{ApiError, auth::Identity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use storage_engine::{CommitRequest, Engine, NamespaceOperation};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommitWorkspace {
    pub workspace_id: String,
    pub operation_id: String,
    pub expected_head: String,
    pub fencing_token: u64,
    pub message: String,
    pub request_digest: Option<String>,
}

/// The frozen public envelope uses decimal strings deliberately: their exact
/// spelling participates in the canonical request digest and binding checks.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCommitWorkspace {
    pub workspace_id: String,
    pub operation_id: String,
    pub repository_id: String,
    pub branch_id: String,
    pub expected_head: String,
    pub fencing_token: String,
    pub message: String,
    pub requested_protection: String,
    pub file_placement: BTreeMap<String, FilePlacement>,
    pub expected_topology_generation: String,
    pub sealed_manifest_hash: String,
    pub request_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilePlacement {
    pub primary_slot_id: String,
    pub target_copies: u32,
}

pub struct StageNamespace {
    pub workspace_id: String,
    pub change: NamespaceOperation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateBranch {
    pub repository_id: String,
    pub source_commit_id: String,
    pub name: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveBranch {
    pub branch_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergeLink {
    pub history_tree_id: String,
    pub source_branch_id: String,
    pub source_commit_id: String,
    pub target_branch_id: String,
    pub target_commit_id: String,
    pub operation_id: String,
}

pub struct NamespaceBatch {
    pub slot_id: String,
    pub request: storage_contract::DirectoryNamespaceBatch,
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitPlan {
    pub plan_id: String,
    pub operation_id: String,
    pub plan_digest: String,
}

#[derive(Debug, Clone)]
pub struct FileLocation {
    pub repository_id: String,
    pub path: String,
}

pub struct MergeFileBranches {
    pub file_id: String,
    pub inputs: Vec<storage_contract::FileMergeInput>,
    pub chosen_node_id: String,
    pub surviving_branch_id: String,
    pub confirmed: bool,
    pub location: Option<FileLocation>,
    pub operation_id: String,
}

/// Wire commands accept only canonical decimal strings for generations.
/// These mutations bind their canonical request in the core, not to a lossy
/// reserialization of a public envelope.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProtectionPolicy {
    #[serde(deserialize_with = "nonempty_string")]
    pub policy_id: String,
    #[serde(deserialize_with = "nonempty_string")]
    pub repository_id: String,
    pub path_prefix: String,
    pub target_copies: u32,
    #[serde(with = "storage_contract::u64_string")]
    pub expected_generation: u64,
    #[serde(deserialize_with = "nonempty_string")]
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlacementRule {
    #[serde(deserialize_with = "nonempty_string")]
    pub rule_id: String,
    #[serde(deserialize_with = "nonempty_string")]
    pub repository_id: String,
    pub path_prefix: String,
    #[serde(deserialize_with = "nonempty_string")]
    pub primary_slot_id: String,
    #[serde(with = "storage_contract::u64_string")]
    pub expected_generation: u64,
    #[serde(deserialize_with = "nonempty_string")]
    pub operation_id: String,
}

fn nonempty_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        return Err(serde::de::Error::custom("expected nonempty string"));
    }
    Ok(value)
}

/// This service borrows a locked engine. It has no transport, JSON dispatch, or
/// caller-supplied principal; the authenticated identity is bound at creation.
pub struct Application<'a> {
    engine: &'a mut Engine,
    identity: &'a Identity,
}

impl<'a> Application<'a> {
    pub fn new(engine: &'a mut Engine, identity: &'a Identity) -> Self {
        Self { engine, identity }
    }

    pub fn update_protection_policy(
        &mut self,
        command: UpdateProtectionPolicy,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        authorize_access(
            self.identity,
            repository(self.engine, &command.repository_id)?,
            RepositoryAccess::Manage,
        )?;
        self.engine
            .update_protection_policy(storage_engine::ProtectionPolicyUpdate {
                policy_id: &command.policy_id,
                repository_id: &command.repository_id,
                path_prefix: &command.path_prefix,
                target_copies: command.target_copies,
                expected_generation: command.expected_generation,
                principal: &self.identity.principal,
                operation_id: &command.operation_id,
            })
            .map_err(error)
    }

    pub fn update_placement_rule(
        &mut self,
        command: UpdatePlacementRule,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        authorize_access(
            self.identity,
            repository(self.engine, &command.repository_id)?,
            RepositoryAccess::Manage,
        )?;
        self.engine
            .update_placement_rule(storage_engine::PlacementRuleUpdate {
                rule_id: &command.rule_id,
                repository_id: &command.repository_id,
                path_prefix: &command.path_prefix,
                primary_slot_id: &command.primary_slot_id,
                expected_generation: command.expected_generation,
                principal: &self.identity.principal,
                operation_id: &command.operation_id,
            })
            .map_err(error)
    }

    fn commit_request(&self, command: CommitWorkspace) -> Result<CommitRequest, ApiError> {
        workspace_authorize(self.engine, self.identity, &command.workspace_id)?;
        Ok(CommitRequest {
            workspace_id: command.workspace_id,
            operation_id: command.operation_id,
            principal: self.identity.principal.clone(),
            expected_head: command.expected_head,
            fencing_token: command.fencing_token,
            message: command.message,
            request_digest: command.request_digest,
        })
    }

    pub fn commit_workspace(
        &mut self,
        command: CommitWorkspace,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        let request = self.commit_request(command)?;
        self.engine.commit_workspace(request).map_err(error)
    }

    fn public_commit_request(
        &self,
        command: PublicCommitWorkspace,
    ) -> Result<(CommitRequest, serde_json::Value), ApiError> {
        let fencing_token = storage_contract::parse_u64_decimal(&command.fencing_token)
            .map_err(ApiError::invalid)?;
        let body = serde_json::to_value(&command).map_err(ApiError::invalid)?;
        let request = self.commit_request(CommitWorkspace {
            workspace_id: command.workspace_id,
            operation_id: command.operation_id,
            expected_head: command.expected_head,
            fencing_token,
            message: command.message,
            request_digest: Some(command.request_digest),
        })?;
        Ok((request, body))
    }

    pub fn commit_workspace_public(
        &mut self,
        command: PublicCommitWorkspace,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        let (request, body) = self.public_commit_request(command)?;
        self.engine
            .commit_workspace_wire(request, body)
            .map_err(error)
    }

    pub fn prepare_commit_workspace(
        &mut self,
        command: CommitWorkspace,
    ) -> Result<storage_engine::WorkspaceCommitPreparation, ApiError> {
        let request = self.commit_request(command)?;
        self.engine
            .prepare_workspace_commit(request, None)
            .map_err(error)
    }

    pub fn prepare_commit_workspace_public(
        &mut self,
        command: PublicCommitWorkspace,
    ) -> Result<storage_engine::WorkspaceCommitPreparation, ApiError> {
        let (request, body) = self.public_commit_request(command)?;
        self.engine
            .prepare_workspace_commit(request, Some(body))
            .map_err(error)
    }

    pub fn finish_workspace_commit(
        &mut self,
        workspace: &str,
        prepared: storage_engine::PreparedWorkspaceCommit,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        workspace_authorize(self.engine, self.identity, workspace)?;
        self.engine.finish_workspace_commit(prepared).map_err(error)
    }

    pub fn stage_namespace(
        &mut self,
        command: StageNamespace,
    ) -> Result<storage_engine::Workspace, ApiError> {
        workspace_authorize(self.engine, self.identity, &command.workspace_id)?;
        self.engine
            .namespace_operation(&command.workspace_id, command.change)
            .map_err(error)
    }

    pub fn create_branch(
        &mut self,
        command: CreateBranch,
    ) -> Result<storage_contract::Branch, ApiError> {
        let repo = repository(self.engine, &command.repository_id)?;
        authorize_access(self.identity, repo, RepositoryAccess::Manage)?;
        if self
            .engine
            .state()
            .commits
            .get(&command.source_commit_id)
            .is_none_or(|commit| commit.history_tree_id != repo.history_tree_id)
        {
            return Err(ApiError::new(
                404,
                "HISTORY_NODE_NOT_FOUND",
                "source commit is outside the repository",
            ));
        }
        self.engine
            .create_branch(
                &command.source_commit_id,
                &command.name,
                &self.identity.principal,
                &command.operation_id,
            )
            .map_err(error)
    }

    pub fn archive_branch(
        &mut self,
        command: ArchiveBranch,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        authorize_access(
            self.identity,
            repo_for_branch(self.engine, &command.branch_id)?,
            RepositoryAccess::Manage,
        )?;
        self.engine
            .archive_branch(
                &command.branch_id,
                &self.identity.principal,
                &command.operation_id,
            )
            .map_err(error)
    }

    pub fn merge_link(
        &mut self,
        command: MergeLink,
    ) -> Result<storage_contract::MergeLink, ApiError> {
        authorize_access(
            self.identity,
            repo_for_history(self.engine, &command.history_tree_id)?,
            RepositoryAccess::Manage,
        )?;
        for branch in [&command.source_branch_id, &command.target_branch_id] {
            if repo_for_branch(self.engine, branch)?.history_tree_id != command.history_tree_id {
                return Err(ApiError::forbidden());
            }
        }
        self.engine
            .add_merge_link(
                &command.source_branch_id,
                &command.source_commit_id,
                &command.target_branch_id,
                &command.target_commit_id,
                &self.identity.principal,
                &command.operation_id,
            )
            .map_err(error)
    }

    pub fn namespace_batch(
        &mut self,
        command: NamespaceBatch,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        let disk = self
            .engine
            .config()
            .disks
            .iter()
            .find(|disk| disk.slot_id == command.slot_id)
            .ok_or_else(|| ApiError::new(404, "DISK_NOT_FOUND", "disk not found"))?;
        if disk.level != 1 {
            return Err(ApiError::new(
                403,
                "DISK_READ_ONLY",
                "backup disks are read only",
            ));
        }
        let mut repository_id = None;
        for item in &command.request.items {
            for branch_id in std::iter::once(item.source.directory_branch_id.as_str()).chain(
                item.target
                    .iter()
                    .map(|target| target.directory_branch_id.as_str()),
            ) {
                let branch = self
                    .engine
                    .state()
                    .directory_branches
                    .get(branch_id)
                    .ok_or_else(|| {
                        ApiError::new(
                            404,
                            "DIRECTORY_BRANCH_NOT_FOUND",
                            "directory branch missing",
                        )
                    })?;
                let repo = repository(self.engine, &branch.repository_id)?;
                authorize_access(self.identity, repo, RepositoryAccess::Write)?;
                if repo.primary_slot_id != command.slot_id {
                    return Err(ApiError::invalid(
                        "batch branches must belong to this primary disk",
                    ));
                }
                if repository_id
                    .as_ref()
                    .is_some_and(|id| id != &repo.repository_id)
                {
                    return Err(ApiError::invalid(
                        "batch branches must belong to one repository",
                    ));
                }
                repository_id = Some(repo.repository_id.clone());
            }
        }
        let repository_id =
            repository_id.ok_or_else(|| ApiError::invalid("batch items cannot be empty"))?;
        self.engine
            .directory_namespace_batch(
                &repository_id,
                &command.request,
                &self.identity.principal,
                &command.operation_id,
            )
            .map_err(error)
    }

    pub(crate) fn authorize_file_location(
        &self,
        file: &str,
        location: Option<&FileLocation>,
    ) -> Result<(), ApiError> {
        let version = self
            .engine
            .state()
            .versions
            .values()
            .find(|version| version.file_id == file)
            .ok_or_else(|| ApiError::new(404, "FILE_NOT_FOUND", "file not found"))?;
        let repo = repo_for_history(self.engine, &version.history_tree_id)?;
        authorize_access(self.identity, repo, RepositoryAccess::Write)?;
        if let Some(location) = location {
            name_policy::validate_path(&location.path).map_err(ApiError::invalid)?;
            if location.repository_id != repo.repository_id
                || !self
                    .engine
                    .list_tree(&repo.main_branch_id)
                    .map_err(error)?
                    .iter()
                    .any(|entry| {
                        entry.kind == "file"
                            && entry.file_id == file
                            && entry.lookup_key == name_policy::path_key(&location.path)
                    })
            {
                return Err(ApiError::new(
                    404,
                    "FILE_NOT_FOUND",
                    "file is not present at this logical path",
                ));
            }
        }
        Ok(())
    }

    pub fn merge_file_branches(
        &mut self,
        command: MergeFileBranches,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        self.authorize_file_location(&command.file_id, command.location.as_ref())?;
        if !command.confirmed {
            return Err(ApiError::invalid(
                "confirm the chosen result and surviving branch before merging",
            ));
        }
        self.engine
            .merge_file_branches(
                &command.file_id,
                &command.inputs,
                &command.chosen_node_id,
                &command.surviving_branch_id,
                &self.identity.principal,
                &command.operation_id,
            )
            .map_err(error)
    }

    /// Destructive purge/clear and split plans retain owner, digest and ACL
    /// checks at execution time, even when prepared by a different adapter.
    pub fn commit_plan(
        &mut self,
        command: CommitPlan,
    ) -> Result<storage_engine::OperationResult, ApiError> {
        let plan = self.engine.plan(&command.plan_id).map_err(error)?;
        if plan.principal != self.identity.principal && !self.identity.admin {
            return Err(ApiError::forbidden());
        }
        if command.plan_digest != plan.digest {
            return Err(ApiError::conflict(
                if plan.kind == "split" {
                    "SPLIT_PLAN_STALE"
                } else {
                    "PLAN_STALE"
                },
                "plan digest does not match",
            ));
        }
        let history = super::field(&plan.input, "history_tree_id")?;
        authorize_access(
            self.identity,
            repo_for_history(self.engine, history)?,
            if plan.kind == "historical_save" {
                RepositoryAccess::Write
            } else {
                RepositoryAccess::Manage
            },
        )?;
        self.engine
            .commit_plan(
                &command.plan_id,
                &self.identity.principal,
                &command.operation_id,
            )
            .map_err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, engine_backend::EngineBackend};
    use serde_json::json;
    use std::collections::BTreeSet;
    use storage_contract::{Config, new_id};

    fn fixture() -> (
        tempfile::TempDir,
        Engine,
        storage_engine::Repository,
        Identity,
    ) {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let repo = engine
            .create_repository("Application", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        let identity = Identity {
            principal: "owner".into(),
            admin: false,
            repositories: BTreeSet::from([repo.repository_id.clone()]),
        };
        (root, engine, repo, identity)
    }

    #[test]
    fn protection_commands_validate_wire_types_and_unknown_fields() {
        let policy = json!({"policy_id":new_id("policy"),"repository_id":new_id("repo"),
            "path_prefix":"","target_copies":3,"expected_generation":"0","operation_id":new_id("op")});
        let rule = json!({"rule_id":new_id("rule"),"repository_id":new_id("repo"),
            "path_prefix":"","primary_slot_id":"P01","expected_generation":"0","operation_id":new_id("op")});
        for (is_policy, valid) in [(true, policy), (false, rule)] {
            let decode = |value: serde_json::Value| {
                if is_policy {
                    serde_json::from_value::<UpdateProtectionPolicy>(value).map(|_| ())
                } else {
                    serde_json::from_value::<UpdatePlacementRule>(value).map(|_| ())
                }
            };
            assert!(decode(valid.clone()).is_ok());
            for generation in [
                json!(0),
                json!(null),
                json!(""),
                json!("00"),
                json!("01"),
                json!("+1"),
                json!(" 1"),
                json!("-1"),
                json!("18446744073709551616"),
            ] {
                let mut invalid = valid.clone();
                invalid["expected_generation"] = generation;
                assert!(decode(invalid).is_err());
            }
            let mut max = valid.clone();
            max["expected_generation"] = json!(u64::MAX.to_string());
            assert!(decode(max).is_ok());
            for key in ["repository_id", "operation_id"] {
                let mut invalid = valid.clone();
                invalid[key] = json!("");
                assert!(decode(invalid).is_err());
            }
            let mut extra = valid.clone();
            extra["principal"] = json!("admin");
            assert!(decode(extra).is_err());
            let mut invalid = valid.clone();
            invalid["path_prefix"] = json!(null);
            assert!(decode(invalid).is_err());
            if is_policy {
                for copies in [json!("3"), json!(3.5), json!(-1), json!(4294967296_u64)] {
                    let mut invalid = valid.clone();
                    invalid["target_copies"] = copies;
                    assert!(decode(invalid).is_err());
                }
            }
        }
    }

    #[test]
    fn protection_commands_preserve_core_replay_and_transport_contract() {
        for is_policy in [true, false] {
            let (_root, mut engine, repo, owner) = fixture();
            let operation = new_id("op");
            let id = new_id(if is_policy { "policy" } else { "rule" });
            let (action, input, original) = if is_policy {
                let original = engine
                    .update_protection_policy(storage_engine::ProtectionPolicyUpdate {
                        policy_id: &id,
                        repository_id: &repo.repository_id,
                        path_prefix: "",
                        target_copies: 3,
                        expected_generation: 0,
                        principal: &owner.principal,
                        operation_id: &operation,
                    })
                    .unwrap();
                (
                    crate::BackendAction::UpdatePolicy,
                    json!({"policy_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                        "target_copies":3,"expected_generation":"0","operation_id":operation}),
                    original,
                )
            } else {
                let original = engine
                    .update_placement_rule(storage_engine::PlacementRuleUpdate {
                        rule_id: &id,
                        repository_id: &repo.repository_id,
                        path_prefix: "",
                        primary_slot_id: "P01",
                        expected_generation: 0,
                        principal: &owner.principal,
                        operation_id: &operation,
                    })
                    .unwrap();
                (
                    crate::BackendAction::UpdatePlacementRule,
                    json!({"rule_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                        "primary_slot_id":"P01","expected_generation":"0","operation_id":operation}),
                    original,
                )
            };
            let backend = EngineBackend::new(engine);
            let replay = backend.call(action, &owner, input.clone()).unwrap();
            assert_eq!(replay["request_digest"], original.request_digest);
            assert_eq!(replay["result"], original.result);
            assert_eq!(
                replay["publication_sequence"],
                original.sequence.to_string()
            );
            assert_eq!(replay["operation_kind"], original.kind);
            let mut changed = input.clone();
            changed["path_prefix"] = json!("Other");
            assert_eq!(
                backend.call(action, &owner, changed).unwrap_err().code,
                "IDEMPOTENCY_KEY_REUSED"
            );
            let mut extra = input.clone();
            extra["unexpected"] = json!(true);
            assert_eq!(backend.call(action, &owner, extra).unwrap_err().status, 400);
            let stranger = Identity {
                principal: "stranger".into(),
                admin: false,
                repositories: owner.repositories.clone(),
            };
            assert_eq!(
                backend
                    .call(action, &stranger, input.clone())
                    .unwrap_err()
                    .status,
                403
            );
            let mut malformed = input.clone();
            malformed["expected_generation"] = json!(1);
            assert_eq!(
                backend
                    .call(action, &stranger, malformed.clone())
                    .unwrap_err()
                    .status,
                403
            );
            assert_eq!(
                backend.call(action, &owner, malformed).unwrap_err().status,
                400
            );
            // Transport-independent callers also need both account admission and Manage.
            let mut engine = backend.engine.lock().unwrap();
            let denied = if is_policy {
                Application::new(&mut engine, &stranger)
                    .update_protection_policy(serde_json::from_value(input).unwrap())
            } else {
                Application::new(&mut engine, &stranger)
                    .update_placement_rule(serde_json::from_value(input).unwrap())
            };
            assert_eq!(denied.unwrap_err().status, 403);
            assert_eq!(
                engine.state().operations[&operation].request_digest,
                original.request_digest
            );
            drop(engine);
            let mut update = if is_policy {
                json!({"policy_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                    "target_copies":3,"expected_generation":"1","operation_id":new_id("op")})
            } else {
                json!({"rule_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                    "primary_slot_id":"P01","expected_generation":"1","operation_id":new_id("op")})
            };
            let unadmitted_owner = Identity {
                principal: owner.principal.clone(),
                admin: false,
                repositories: BTreeSet::new(),
            };
            assert_eq!(
                backend
                    .call(action, &unadmitted_owner, update.clone())
                    .unwrap_err()
                    .status,
                403
            );
            let applied = backend.call(action, &owner, update.clone()).unwrap();
            let record = if is_policy { "policy" } else { "rule" };
            assert_eq!(applied["result"][record]["generation"], "2");
            update["operation_id"] = json!(new_id("op"));
            let stale = backend.call(action, &owner, update).unwrap_err();
            assert_eq!(stale.code, "CONFIG_CONFLICT");
            assert_eq!(stale.status, 409);
        }
    }

    #[test]
    fn public_commit_roundtrip_preserves_canonical_envelope_and_executes() {
        let (_root, mut engine, repo, identity) = fixture();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, &identity.principal)
            .unwrap();
        Application::new(&mut engine, &identity)
            .stage_namespace(StageNamespace {
                workspace_id: workspace.workspace_id.clone(),
                change: NamespaceOperation {
                    kind: "mkdir".into(),
                    path: "Folder".into(),
                    destination: None,
                },
            })
            .unwrap();
        let mut body = engine
            .seal_workspace(&workspace.workspace_id, &identity.principal)
            .unwrap();
        body["request_digest"] =
            storage_engine::commit_request_digest(&workspace.workspace_id, &body)
                .unwrap()
                .into();
        let command: PublicCommitWorkspace = serde_json::from_value(body.clone()).unwrap();
        assert_eq!(serde_json::to_value(&command).unwrap(), body);
        let result = Application::new(&mut engine, &identity)
            .commit_workspace_public(command.clone())
            .unwrap();
        let replay = Application::new(&mut engine, &identity)
            .commit_workspace_public(command)
            .unwrap();
        assert_eq!(replay.commit_id, result.commit_id);
        assert!(
            engine
                .list_tree(&repo.main_branch_id)
                .unwrap()
                .iter()
                .any(|entry| entry.path == "Folder")
        );
    }

    #[test]
    fn workspace_ownership_and_account_admission_apply_to_typed_and_wire_paths() {
        let (_root, mut engine, repo, owner) = fixture();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, &owner.principal)
            .unwrap();
        let intruder = Identity {
            principal: "intruder".into(),
            ..owner.clone()
        };
        let result = Application::new(&mut engine, &intruder).stage_namespace(StageNamespace {
            workspace_id: workspace.workspace_id.clone(),
            change: NamespaceOperation {
                kind: "mkdir".into(),
                path: "TypedDenied".into(),
                destination: None,
            },
        });
        assert_eq!(result.unwrap_err().code, "FORBIDDEN");
        let unadmitted_owner = Identity {
            repositories: BTreeSet::new(),
            ..owner.clone()
        };
        let result =
            Application::new(&mut engine, &unadmitted_owner).commit_workspace(CommitWorkspace {
                workspace_id: workspace.workspace_id.clone(),
                operation_id: new_id("op"),
                expected_head: workspace.base_head.clone(),
                fencing_token: workspace.fencing_token,
                message: String::new(),
                request_digest: None,
            });
        assert_eq!(result.unwrap_err().code, "FORBIDDEN");
        let backend = EngineBackend::new(engine);
        for identity in [&intruder, &unadmitted_owner] {
            let result = backend.call(
                crate::BackendAction::Namespace,
                identity,
                json!({"workspace_id":workspace.workspace_id,
                "kind":"mkdir", "path":"WireDenied"}),
            );
            assert_eq!(result.unwrap_err().code, "FORBIDDEN");
        }
        let engine = backend.engine.lock().unwrap();
        assert!(
            engine
                .workspace(&workspace.workspace_id)
                .unwrap()
                .namespace_operations
                .is_empty()
        );
    }

    #[test]
    fn public_commit_decode_rejects_unknown_missing_and_numeric_decimal_fields() {
        let body = json!({"workspace_id":"ws", "operation_id":"op", "repository_id":"repo", "branch_id":"branch",
            "expected_head":"head", "fencing_token":"1", "message":"", "requested_protection":"strict",
            "file_placement":{}, "expected_topology_generation":"1", "sealed_manifest_hash":"hash", "request_digest":"digest"});
        assert!(serde_json::from_value::<PublicCommitWorkspace>(body.clone()).is_ok());
        for field in [
            "workspace_id",
            "operation_id",
            "expected_head",
            "message",
            "request_digest",
        ] {
            let mut missing = body.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<PublicCommitWorkspace>(missing).is_err(),
                "{field}"
            );
        }
        for field in ["fencing_token", "expected_topology_generation"] {
            let mut numeric = body.clone();
            numeric[field] = json!(1);
            assert!(
                serde_json::from_value::<PublicCommitWorkspace>(numeric).is_err(),
                "{field}"
            );
        }
        let mut extra = body;
        extra["principal"] = "someone-else".into();
        assert!(serde_json::from_value::<PublicCommitWorkspace>(extra).is_err());
    }

    #[test]
    fn typed_branch_requires_manage_and_rejects_cross_repository_source() {
        let (_root, mut engine, repo, owner) = fixture();
        let other = engine
            .create_repository("Other", "P01", 3, &owner.principal, &new_id("op"))
            .unwrap();
        let source = engine.state().branches[&other.main_branch_id]
            .head_commit_id
            .clone();
        let result = Application::new(&mut engine, &owner).create_branch(CreateBranch {
            repository_id: repo.repository_id,
            source_commit_id: source,
            name: "Cross scope".into(),
            operation_id: new_id("op"),
        });
        assert_eq!(result.unwrap_err().code, "HISTORY_NODE_NOT_FOUND");
    }
    #[test]
    fn upload_finalization_rechecks_completed_account_revocation() {
        for disabled in [false, true] {
            let (root, mut engine, repo, owner) = fixture();
            let workspace = engine
                .create_workspace(&repo.main_branch_id, &owner.principal)
                .unwrap();
            let source = root.path().join("upload-source");
            std::fs::write(&source, b"prepared bytes").unwrap();
            let item = new_id("entry");
            let backend = std::sync::Arc::new(EngineBackend::new(engine));
            let mut auth = crate::auth::AuthStore::initialize(
                root.path(),
                "admin",
                "integration-test-password",
            )
            .unwrap();
            auth.upsert_account(
                "owner",
                Some("integration-test-password"),
                false,
                owner.repositories.clone(),
                false,
            )
            .unwrap();
            let mut auth = backend.protect_auth(auth).unwrap();
            let plan = {
                let engine = backend.engine.lock().unwrap();
                engine
                    .prepare_upload(&workspace.workspace_id, &item, "file.txt", &source, 1)
                    .unwrap()
            };
            // This update completes while byte preparation holds no engine lock.
            auth.upsert_account(
                "owner",
                None,
                false,
                if disabled {
                    owner.repositories.clone()
                } else {
                    BTreeSet::new()
                },
                disabled,
            )
            .unwrap();
            let prepared = plan.execute().unwrap();
            let mut engine = backend.engine.lock().unwrap();
            let result = crate::engine_backend::finish_authorized_upload(
                &mut engine,
                &owner,
                &workspace.workspace_id,
                prepared,
            );
            assert_eq!(
                result.unwrap_err().code,
                if disabled {
                    "UNAUTHORIZED"
                } else {
                    "FORBIDDEN"
                }
            );
            assert!(
                engine
                    .workspace(&workspace.workspace_id)
                    .unwrap()
                    .files
                    .is_empty()
            );
        }
    }
    #[test]
    fn commit_preparation_releases_engine_and_rechecks_completed_revocation() {
        use std::sync::{Arc, Mutex, mpsc};
        use std::time::Duration;
        for public in [false, true] {
            for revoke in [None, Some(false), Some(true)] {
                let (root, mut engine, repo, owner) = fixture();
                let workspace = engine
                    .create_workspace(&repo.main_branch_id, &owner.principal)
                    .unwrap();
                engine
                    .stage_bytes(&workspace.workspace_id, "file.txt", b"commit bytes")
                    .unwrap();
                let operation = new_id("op");
                let input = if public {
                    let mut input = engine
                        .seal_workspace(&workspace.workspace_id, &owner.principal)
                        .unwrap();
                    input["operation_id"] = operation.clone().into();
                    input["request_digest"] =
                        storage_engine::commit_request_digest(&workspace.workspace_id, &input)
                            .unwrap()
                            .into();
                    input
                } else {
                    json!({"workspace_id":workspace.workspace_id,"operation_id":operation,
                        "expected_head":workspace.base_head,"fencing_token":workspace.fencing_token.to_string(),"message":"commit"})
                };
                let backend = Arc::new(EngineBackend::new(engine));
                let mut auth = crate::auth::AuthStore::initialize(
                    root.path(),
                    "admin",
                    "integration-test-password",
                )
                .unwrap();
                auth.upsert_account(
                    "owner",
                    Some("integration-test-password"),
                    false,
                    owner.repositories.clone(),
                    false,
                )
                .unwrap();
                let mut auth = backend.protect_auth(auth).unwrap();
                // A request can retain an Identity captured before revocation.
                // Reject it before preparing bytes, even though concurrent auth
                // writes would also be fenced once preparation has started.
                if let Some(disabled) = revoke {
                    auth.upsert_account(
                        "owner",
                        None,
                        false,
                        if disabled {
                            owner.repositories.clone()
                        } else {
                            BTreeSet::new()
                        },
                        disabled,
                    )
                    .unwrap();
                    let result = backend.call(
                        if public {
                            crate::BackendAction::CommitWorkspace
                        } else {
                            crate::BackendAction::CommitWorkspaceInternal
                        },
                        &owner,
                        input,
                    );
                    assert_eq!(
                        result.unwrap_err().code,
                        if disabled {
                            "UNAUTHORIZED"
                        } else {
                            "FORBIDDEN"
                        }
                    );
                    let engine = backend.engine.lock().unwrap();
                    assert!(!engine.state().operations.contains_key(&operation));
                    assert!(engine.list_tree(&repo.main_branch_id).unwrap().is_empty());
                    continue;
                }
                let (started_tx, started_rx) = mpsc::channel();
                let (release_tx, release_rx) = mpsc::channel();
                let release_rx = Mutex::new(release_rx);
                *backend.commit_preparation_hook.lock().unwrap() = Some(Arc::new(move || {
                    started_tx.send(()).unwrap();
                    release_rx.lock().unwrap().recv().unwrap();
                }));
                let writer_backend = backend.clone();
                let writer_owner = owner.clone();
                let writer = std::thread::spawn(move || {
                    writer_backend.call(
                        if public {
                            crate::BackendAction::CommitWorkspace
                        } else {
                            crate::BackendAction::CommitWorkspaceInternal
                        },
                        &writer_owner,
                        input,
                    )
                });
                started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                let reader_backend = backend.clone();
                let reader_owner = owner.clone();
                let (query_tx, query_rx) = mpsc::channel();
                let reader = std::thread::spawn(move || {
                    query_tx
                        .send(reader_backend.call(
                            crate::BackendAction::Repositories,
                            &reader_owner,
                            json!({}),
                        ))
                        .unwrap();
                });
                let queried = query_rx.recv_timeout(Duration::from_secs(2));
                if queried.is_err() {
                    release_tx.send(()).unwrap();
                    let _ = writer.join();
                    let _ = reader.join();
                    panic!("repository query blocked behind detached commit preparation");
                }
                assert!(queried.unwrap().is_ok());
                reader.join().unwrap();
                let before_busy = auth.snapshot().unwrap();
                let busy = auth
                    .upsert_account("owner", None, false, owner.repositories.clone(), false)
                    .unwrap_err();
                assert_eq!(busy.code, "COMMIT_PREPARATION_BUSY");
                assert!(busy.retryable);
                assert_eq!(auth.snapshot().unwrap(), before_busy);
                assert!(auth.identity_for_principal("admin").is_ok());
                release_tx.send(()).unwrap();
                let result = writer.join().unwrap();
                let engine = backend.engine.lock().unwrap();
                assert!(result.is_ok(), "{result:?}");
                assert!(engine.state().operations.contains_key(&operation));
            }
        }
    }
}
