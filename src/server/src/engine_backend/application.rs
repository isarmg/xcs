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
use data_serialization::{Deserialize, Serialize};
use std::collections::BTreeMap;
use storage_engine::{CommitRequest, Engine, NamespaceOperation};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serialization(deny_unknown_fields)]
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
#[serialization(deny_unknown_fields)]
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
#[serialization(deny_unknown_fields)]
pub struct FilePlacement {
    pub primary_slot_id: String,
    pub target_copies: u32,
}

pub struct StageNamespace {
    pub workspace_id: String,
    pub change: NamespaceOperation,
}

#[derive(Debug, Clone, Deserialize)]
#[serialization(deny_unknown_fields)]
pub struct CreateBranch {
    pub repository_id: String,
    pub source_commit_id: String,
    pub name: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serialization(deny_unknown_fields)]
pub struct ArchiveBranch {
    pub branch_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serialization(deny_unknown_fields)]
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
#[serialization(deny_unknown_fields)]
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
#[serialization(deny_unknown_fields)]
pub struct UpdateProtectionPolicy {
    #[serialization(deserialize_with = "nonempty_string")]
    pub policy_id: String,
    #[serialization(deserialize_with = "nonempty_string")]
    pub repository_id: String,
    pub path_prefix: String,
    pub target_copies: u32,
    #[serialization(with = "storage_contract::u64_string")]
    pub expected_generation: u64,
    #[serialization(deserialize_with = "nonempty_string")]
    pub operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serialization(deny_unknown_fields)]
pub struct UpdatePlacementRule {
    #[serialization(deserialize_with = "nonempty_string")]
    pub rule_id: String,
    #[serialization(deserialize_with = "nonempty_string")]
    pub repository_id: String,
    pub path_prefix: String,
    #[serialization(deserialize_with = "nonempty_string")]
    pub primary_slot_id: String,
    #[serialization(with = "storage_contract::u64_string")]
    pub expected_generation: u64,
    #[serialization(deserialize_with = "nonempty_string")]
    pub operation_id: String,
}

fn nonempty_string<'de, D: data_serialization::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        return Err(data_serialization::de::Error::custom(
            "expected nonempty string",
        ));
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
    ) -> Result<(CommitRequest, json_codec::Value), ApiError> {
        let fencing_token = storage_contract::parse_u64_decimal(&command.fencing_token)
            .map_err(ApiError::invalid)?;
        let body = json_codec::to_value(&command).map_err(ApiError::invalid)?;
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
#[path = "../../../../test/server/unit/engine_backend/application/tests.rs"]
mod tests;
