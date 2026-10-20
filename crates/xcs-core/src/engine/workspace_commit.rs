//! Owned, non-publishing workspace preparation, followed by one serialized decision.
use super::{DetachedPublication, Engine, MutationPlan};
use crate::*;
use std::{
    fs::File,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct PreparationAdmission(Arc<AtomicBool>);
impl Drop for PreparationAdmission {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Private planning view. It exposes no publication or mutation entry point.
/// One per-engine admission bounds the extra snapshot State clone; payload
/// workers share this view rather than cloning State per file.
struct PlanningSnapshot {
    engine: Engine,
}

pub struct WorkspaceCommitPreparation {
    snapshot: PlanningSnapshot,
    request: CommitRequest,
    wire_body: Option<serde_json::Value>,
    frozen_workspace: Vec<u8>,
    admission: PreparationAdmission,
}

pub struct PreparedWorkspaceCommit {
    publication: Option<DetachedPublication>,
    existing: Option<OperationResult>,
    workspace: Workspace,
    request: CommitRequest,
    wire: Option<crate::wire::WireCommit>,
    expected_digest: String,
    frozen_workspace: Vec<u8>,
    base_root: std::path::PathBuf,
    base_topology: u64,
    base_business: u64,
    base_snapshot: Option<String>,
    base_slot: u64,
    base_sequence: u64,
    frozen_config: Vec<u8>,
    _locks: Vec<File>,
    _admission: Option<PreparationAdmission>,
}

impl Engine {
    /// Capture authorization and metadata under the Engine mutex. No payload
    /// bytes are read here. execute() owns all expensive planning/copy/hash work.
    pub fn prepare_workspace_commit(
        &self,
        request: CommitRequest,
        wire_body: Option<serde_json::Value>,
    ) -> Result<WorkspaceCommitPreparation> {
        let workspace = self.workspace(&request.workspace_id)?;
        if workspace.principal != request.principal {
            return Err(Error::new(
                "PERMISSION_DENIED",
                "workspace belongs to another principal",
            ));
        }
        let replay = self.state.operations.contains_key(&request.operation_id);
        self.require_grant(
            self.repository_for_branch(&workspace.branch_id)?,
            &request.principal,
            if replay { "read" } else { "write" },
        )?;
        if !replay {
            self.require_write()?;
            if workspace.fencing_token != request.fencing_token {
                return Err(Error::new(
                    "FENCING_TOKEN_INVALID",
                    "workspace fencing token differs",
                ));
            }
        }
        self.commit_preparation
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                Error::new(
                    "COMMIT_PREPARATION_BUSY",
                    "another workspace commit is being prepared",
                )
                .retryable()
            })?;
        let admission = PreparationAdmission(self.commit_preparation.clone());
        // Duplicate the existing open-file descriptions: they keep this pool's
        // process locks alive even if the original Engine is dropped meanwhile.
        let locks = self
            ._locks
            .iter()
            .map(File::try_clone)
            .collect::<std::io::Result<Vec<_>>>()?;
        let engine = Engine {
            root: self.root.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            workspaces: self.workspaces.clone(),
            blocked: self.blocked,
            snapshot: self.snapshot.clone(),
            next_slot: self.next_slot,
            reads: self.reads.clone(),
            upload_preparations: self.upload_preparations.clone(),
            // The frozen view may plan writes; only the live Engine is fenced.
            commit_preparation: Arc::new(AtomicBool::new(false)),
            history_query_proof: std::sync::Mutex::new(super::index::QueryProof::default()),
            _locks: locks,
            fault: None,
        };
        Ok(WorkspaceCommitPreparation {
            snapshot: PlanningSnapshot { engine },
            request,
            wire_body,
            frozen_workspace: canonical_json(&workspace)?,
            admission,
        })
    }

    /// Recheck live authority and the exact planning base before the first
    /// PREPARE/ACCEPT record. A stale preparation is a definite retryable conflict,
    /// never an invented unknown operation or a partially accepted candidate.
    pub fn finish_workspace_commit(
        &mut self,
        mut prepared: PreparedWorkspaceCommit,
    ) -> Result<OperationResult> {
        if self.root != prepared.base_root {
            return Err(Error::new(
                "COMMIT_PREPARATION_STALE",
                "prepared commit belongs to another engine",
            )
            .retryable());
        }
        // The caller holds exclusive access to the live Engine again. Release
        // only our own claim before revalidation; no physical preparation remains.
        drop(prepared._admission.take());
        let workspace = self.workspace(&prepared.request.workspace_id)?;
        if workspace.principal != prepared.request.principal {
            return Err(Error::new("PERMISSION_DENIED", "workspace owner changed"));
        }
        if let Some(existing) = self.state.operations.get(&prepared.request.operation_id) {
            self.require_grant(
                self.repository_for_branch(&workspace.branch_id)?,
                &prepared.request.principal,
                "read",
            )?;
            if existing.request_digest != prepared.expected_digest
                || self
                    .state
                    .operation_principals
                    .get(&prepared.request.operation_id)
                    != Some(&prepared.request.principal)
            {
                return Err(Error::new(
                    "IDEMPOTENCY_KEY_REUSED",
                    "operation id is bound to different input",
                ));
            }
            return self.operation(&prepared.request.operation_id);
        }
        self.require_write()?;
        self.require_grant(
            self.repository_for_branch(&workspace.branch_id)?,
            &prepared.request.principal,
            "write",
        )?;
        if canonical_json(&workspace)? != prepared.frozen_workspace
            || self.snapshot != prepared.base_snapshot
            || self.next_slot != prepared.base_slot
            || self.state.sequence != prepared.base_sequence
            || self.state.topology_generation != prepared.base_topology
            || self.state.business_generation != prepared.base_business
            || canonical_json(&self.config)? != prepared.frozen_config
        {
            return Err(Error::new("COMMIT_PREPARATION_STALE", "workspace or publication base changed during preparation; retry the same operation").retryable());
        }
        if !matches!(workspace.state.as_str(), "open" | "sealed")
            || workspace.fencing_token != prepared.request.fencing_token
        {
            return Err(Error::new(
                "FENCING_TOKEN_INVALID",
                "workspace no longer admits this commit",
            ));
        }
        if crate::business::now().saturating_sub(workspace.created_at)
            > self.config.workspace_ttl_seconds
        {
            return Err(Error::new(
                "FENCING_TOKEN_EXPIRED",
                "workspace lease expired during preparation",
            ));
        }
        // Requests become durable before decision publication, just as in the
        // synchronous path, and retain the original operation id and digest.
        self.write_workspace_commit_request(
            &prepared.workspace,
            &prepared.request,
            prepared.wire.as_ref(),
        )?;
        let result = if let Some(publication) = prepared.publication {
            self.decide_publication(publication)?
        } else {
            prepared
                .existing
                .ok_or_else(|| Error::new("STATE_INVALID", "prepared commit has no publication"))?
        };
        self.complete_workspace_commit(prepared.workspace, result)
    }
}

impl WorkspaceCommitPreparation {
    /// Run on a bounded blocking worker, outside the live Engine mutex.
    pub fn execute(mut self) -> Result<PreparedWorkspaceCommit> {
        let engine = &self.snapshot.engine;
        let plan = if let Some(body) = self.wire_body.take() {
            engine.plan_workspace_commit_wire(self.request, body)?
        } else {
            engine.plan_workspace_commit_inner(self.request, None)?
        };
        let (publication, existing, expected_digest) = match plan.mutation {
            MutationPlan::Existing(result) => (None, Some(result.clone()), result.request_digest),
            MutationPlan::Prepared {
                next,
                prepared,
                result,
            } => {
                let digest = result.request_digest.clone();
                (
                    Some(engine.prepare_publication(
                        next,
                        prepared,
                        result,
                        crate::disk::MAX_METADATA,
                    )?),
                    None,
                    digest,
                )
            }
        };
        let base_root = engine.root.clone();
        let base_topology = engine.state.topology_generation;
        let base_business = engine.state.business_generation;
        let base_snapshot = engine.snapshot.clone();
        let base_slot = engine.next_slot;
        let base_sequence = engine.state.sequence;
        let frozen_config = canonical_json(&engine.config)?;
        let locks = std::mem::take(&mut self.snapshot.engine._locks);
        Ok(PreparedWorkspaceCommit {
            publication,
            existing,
            expected_digest,
            workspace: plan.workspace,
            request: plan.request,
            wire: plan.wire,
            frozen_workspace: self.frozen_workspace,
            base_root,
            base_topology,
            base_business,
            base_snapshot,
            base_slot,
            base_sequence,
            frozen_config,
            _locks: locks,
            _admission: Some(self.admission),
        })
    }
}
