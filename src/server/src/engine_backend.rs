//! Authorization and wire adaptation for the storage engine.
use crate::{ApiError, Backend, Content, auth::Identity};
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};
use storage_engine::{Engine, Grants, NamespaceOperation, Repository};
pub mod application;
use application::Application;
mod content;
mod file_manager;
mod read_projection;
mod wire;
use wire::{
    branch_wire, content_wire, current_entry_source, encode, error, field, keys, operation,
    operation_wire, pending_entry_source, sanitized,
};

pub struct EngineBackend {
    engine: Mutex<Engine>,
    #[cfg(test)]
    commit_preparation_hook: Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
}
impl EngineBackend {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Mutex::new(engine),
            #[cfg(test)]
            commit_preparation_hook: Mutex::new(None),
        }
    }
    pub fn protect_auth(
        self: &std::sync::Arc<Self>,
        mut auth: crate::auth::AuthStore,
    ) -> Result<crate::auth::AuthStore, ApiError> {
        let backend = self.clone();
        auth.attach_backup(std::sync::Arc::new(move |snapshot| {
            let mut engine = backend.engine.lock().map_err(|_| {
                ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart")
            })?;
            if engine.auth_state().as_ref() == Some(&snapshot) {
                return Ok(());
            }
            engine
                .persist_auth_state(snapshot, "system_auth", &storage_engine::new_id("op"))
                .map_err(error)
        }))?;
        Ok(auth)
    }
    pub fn open_auth(
        self: &std::sync::Arc<Self>,
        root: &Path,
    ) -> Result<crate::auth::AuthStore, ApiError> {
        let snapshot = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?
            .auth_state();
        self.protect_auth(crate::auth::AuthStore::open_with_snapshot(root, snapshot)?)
    }
}
fn repository<'a>(engine: &'a Engine, id: &str) -> Result<&'a Repository, ApiError> {
    engine
        .state()
        .repositories
        .get(id)
        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "repository not found"))
}
fn repo_for_history<'a>(engine: &'a Engine, id: &str) -> Result<&'a Repository, ApiError> {
    let history = engine
        .state()
        .histories
        .get(id)
        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "history not found"))?;
    repository(engine, &history.repository_id)
}
fn repo_for_branch<'a>(engine: &'a Engine, id: &str) -> Result<&'a Repository, ApiError> {
    let branch = engine
        .state()
        .branches
        .get(id)
        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "branch not found"))?;
    if branch.state == "migrated" {
        return Err(ApiError::new(
            409,
            "TREE_MOVED",
            "branch has moved to an independent history domain",
        ));
    }
    repo_for_history(engine, &branch.history_tree_id)
}
fn authorize(
    identity: &Identity,
    repo: &Repository,
    write: bool,
    manage: bool,
) -> Result<(), ApiError> {
    authorize_access(
        identity,
        repo,
        if manage {
            RepositoryAccess::Manage
        } else if write {
            RepositoryAccess::Write
        } else {
            RepositoryAccess::Read
        },
    )
}

pub use storage_contract::RepositoryAccess;

pub fn authorize_access(
    identity: &Identity,
    repo: &Repository,
    access: RepositoryAccess,
) -> Result<(), ApiError> {
    if identity.admin {
        return Ok(());
    }
    let admitted = identity.can_access(&repo.repository_id)
        || repo
            .account_scope_repository_id
            .as_deref()
            .is_some_and(|source| identity.can_access(source));
    if !admitted {
        return Err(ApiError::forbidden());
    }
    let allowed = repo.grants.allows(&identity.principal, access);
    if allowed {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}
fn workspace_authorize(
    engine: &Engine,
    identity: &Identity,
    id: &str,
) -> Result<storage_engine::Workspace, ApiError> {
    let workspace = engine.workspace(id).map_err(error)?;
    if workspace.principal != identity.principal && !identity.admin {
        return Err(ApiError::forbidden());
    }
    authorize(
        identity,
        repo_for_branch(engine, &workspace.branch_id)?,
        true,
        false,
    )?;
    Ok(workspace)
}

/// Reauthorize after the unlocked copy against the current protected account
/// snapshot, not the Identity captured before account revocation could publish.
fn current_protected_identity(engine: &Engine, identity: &Identity) -> Result<Identity, ApiError> {
    match engine.auth_state() {
        Some(snapshot) => {
            crate::auth::AuthStore::identity_from_snapshot(snapshot, &identity.principal)
        }
        None => Ok(identity.clone()), // Embedded engines without a protected account store.
    }
}

fn finish_authorized_upload(
    engine: &mut Engine,
    identity: &Identity,
    workspace: &str,
    prepared: storage_engine::PreparedUpload,
) -> Result<Value, ApiError> {
    let current = current_protected_identity(engine, identity)?;
    workspace_authorize(engine, &current, workspace)?;
    sanitized(engine.finish_upload(prepared).map_err(error)?)
}

impl Backend for EngineBackend {
    fn call(
        &self,
        action: crate::BackendAction,
        identity: &Identity,
        input: Value,
    ) -> Result<Value, ApiError> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        match action {
            crate::BackendAction::FileManagerDisks => {
                let access = file_manager::disk_access(&engine, identity);
                let observation = engine.prepare_disk_observation();
                drop(engine);
                let disks = observation.status().map_err(error)?.into_iter().filter_map(|disk| {
                    let writable = access.get(&disk.slot_id)?;
                    Some(json!({"slot_id":disk.slot_id,"level":disk.level,"online":disk.online,"writable":writable,"available_bytes":disk.available_bytes.to_string(),"used_bytes":disk.used_bytes.to_string(),"simulation":disk.simulation}))
                }).collect::<Vec<_>>();
                Ok(json!({"disks":disks}))
            }
            crate::BackendAction::FileManagerBackupConfiguration
            | crate::BackendAction::FileManagerBranch
            | crate::BackendAction::FileManagerConfigureBackups
            | crate::BackendAction::FileManagerDirectoryBranches
            | crate::BackendAction::FileManagerDirectorySelect
            | crate::BackendAction::FileManagerDownloadContext
            | crate::BackendAction::FileManagerEntries
            | crate::BackendAction::FileManagerFileHistory
            | crate::BackendAction::FileManagerFileMetadata
            | crate::BackendAction::FileManagerHistory
            | crate::BackendAction::FileManagerMerge
            | crate::BackendAction::FileManagerNamespace
            | crate::BackendAction::FileManagerNamespaceBatch
            | crate::BackendAction::FileManagerNodeNote
            | crate::BackendAction::FileManagerRenameBranch
            | crate::BackendAction::FileManagerSelect
            | crate::BackendAction::FileManagerTags
            | crate::BackendAction::FileManagerVersionMetadata
            | crate::BackendAction::FileManagerWriteSession => {
                file_manager::call(&mut engine, identity, action, input)
            }
            crate::BackendAction::MonitorDisks => {
                identity.require_admin()?;
                let observation = engine.prepare_disk_observation();
                drop(engine);
                let mut disks = Vec::new();
                for disk in observation.status().map_err(error)? {
                    let capacity = observation.capacity(&disk.slot_id).ok();
                    let (power_watts, power_unavailable_reason) = crate::telemetry::disk_power(
                        &disk.physical_device,
                        disk.simulation,
                        disk.online,
                    );
                    let usage_percent = capacity
                        .filter(|capacity| capacity.0 > 0)
                        .map(|capacity| capacity.2 as f64 / capacity.0 as f64 * 100.0);
                    disks.push(json!({"id":disk.slot_id,"name":disk.slot_id,"online":disk.online,"read_only":disk.level!=1,"simulation":disk.simulation,
                        "total_bytes":capacity.map(|c|c.0.to_string()),"available_bytes":capacity.map(|c|c.1.to_string()),"used_bytes":capacity.map(|c|c.2.to_string()),"usage_percent":usage_percent,"power_watts":power_watts,"power_unavailable_reason":power_unavailable_reason}));
                }
                Ok(json!({"disks":disks}))
            }
            crate::BackendAction::Capabilities => Ok(
                json!({"name_policy":"PortableName","path_policy":"PortablePath","unicode_data_version":"16.0.0","supported_save_modes":["full"],"full_only":true,"full_branches":false,"branch_creation":"record-until-write","branch_content":"inherit-parent-until-first-write","file_branches":true,"branch_scope":"file","directory_branches":true,"directory_branch_creation":"inherit-directory-tree","directory_subtrees":true,"namespace_batch":true,"smb_default_branch":"selected-root-directory","file_manager":true,"historical_save_creates_branch":true,"merge_behavior":"explicit-file-node-adoption","file_branch_merge":true,"directory_branch_merge":false,"directory_merge_status":"design-pending","file_node_tags":true,"backup_configuration":true,"range_upload":false,"complete_upload":true,"max_upload_bytes":engine.config().max_file_bytes.to_string(),"authentication":"session-bearer-or-cookie","platforms":{"web":true,"smb":true},"operations":["repositories","tree","workspace","commit","history","graph","branch","merge-link","file-branch-merge","file-node-tags","backup-configuration","purge-plan","split-plan","version-range-read","file-manager","file-branches","file-branch-rename","file-path-nodes","directory-branches","directory-subtrees","namespace-batch","default-branch","file-notes","system-status"]}),
            ),
            crate::BackendAction::Ready => {
                if engine.is_read_only() {
                    return Err(ApiError::new(
                        503,
                        "RECOVERY_REQUIRED",
                        "storage is open for verified recovery reads only",
                    ));
                }
                let mode = engine.config().mode.clone();
                let sequence = engine.state().sequence;
                let observation = engine.prepare_disk_observation();
                drop(engine);
                let status = observation.status().map_err(error)?;
                let members = status
                    .iter()
                    .filter(|disk| disk.metadata_member && disk.online)
                    .count();
                if members < 3 {
                    return Err(ApiError::new(
                        503,
                        "METADATA_UNAVAILABLE",
                        "all three metadata members are required for normal writes",
                    ));
                }
                Ok(json!({"ready":true,"mode":mode,"sequence":sequence.to_string()}))
            }
            crate::BackendAction::Repositories => {
                let values = engine
                    .state()
                    .repositories
                    .values()
                    .filter(|repo| authorize(identity, repo, false, false).is_ok())
                    .collect::<Vec<_>>();
                Ok(json!({"repositories":values}))
            }
            crate::BackendAction::Repository => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, false, false)?;
                encode(repo)
            }
            crate::BackendAction::Branches => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, false, false)?;
                Ok(
                    json!({"branches":engine.state().branches.values().filter(|branch|branch.history_tree_id==repo.history_tree_id).map(|branch| branch_wire(&engine, branch)).collect::<Result<Vec<_>, _>>()?}),
                )
            }
            crate::BackendAction::Tree => {
                let projection = read_projection::Tree::capture(&engine, identity, &input)?;
                read_projection::render_unlocked(engine, projection)
            }
            crate::BackendAction::SmbBinding => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, false, true)?;
                let history = engine
                    .state()
                    .histories
                    .get(&repo.history_tree_id)
                    .ok_or_else(|| {
                        ApiError::new(500, "STATE_INVALID", "repository history is missing")
                    })?;
                Ok(
                    json!({"repository_id":repo.repository_id,"branch_id":repo.main_branch_id,"enabled":repo.smb_enabled,"history_generation":history.generation.to_string()}),
                )
            }
            crate::BackendAction::DefaultBranch => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, true, false)?;
                let selected = engine.state().directory_selections.get(&repo.repository_id);
                let name = selected
                    .and_then(|id| engine.state().directory_branches.get(id))
                    .map(|branch| branch.name.as_str())
                    .unwrap_or("main");
                let branches = engine
                    .directory_branches(&repo.repository_id, "")
                    .map_err(error)?;
                Ok(
                    json!({"repository_id":repo.repository_id,"namespace_branch_id":repo.main_branch_id,
                    "directory_branch_id":selected,"name":name,"directory_branches":branches}),
                )
            }
            crate::BackendAction::DirectoryBranchScopes => file_manager::directory_branch_scopes(
                &engine,
                identity,
                field(&input, "repository_id")?,
                input
                    .get("query")
                    .and_then(|query| query.get("path"))
                    .and_then(Value::as_str),
            ),
            crate::BackendAction::InitializeDirectoryScope => {
                keys(&input, &["repository_id", "path", "operation_id"])?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, false)?;
                // The empty path explicitly identifies the repository root.
                let path = input
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::invalid("path must be a string"))?;
                if !path.is_empty() {
                    name_policy::validate_path(path).map_err(ApiError::invalid)?;
                }
                let result = engine
                    .initialize_directory_scope(repo, path, &identity.principal, operation(&input)?)
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::UpdateDefaultBranch => {
                keys(
                    &input,
                    &["repository_id", "directory_branch_id", "operation_id"],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, false)?;
                let result = engine
                    .select_default_directory_branch(
                        repo,
                        field(&input, "directory_branch_id")?,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::RenameDirectoryBranch => {
                keys(
                    &input,
                    &[
                        "repository_id",
                        "directory_branch_id",
                        "name",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, false)?;
                let result = engine
                    .rename_directory_branch(
                        repo,
                        field(&input, "directory_branch_id")?,
                        field(&input, "name")?,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::UpdateSmbBinding => {
                identity.require_admin()?;
                keys(
                    &input,
                    &[
                        "repository_id",
                        "branch_id",
                        "enabled",
                        "expected_history_generation",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, true)?;
                let enabled = input
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| ApiError::invalid("enabled must be a boolean"))?;
                let generation = storage_contract::parse_u64_decimal(field(
                    &input,
                    "expected_history_generation",
                )?)
                .map_err(ApiError::invalid)?;
                let result = engine
                    .set_smb_binding(
                        repo,
                        field(&input, "branch_id")?,
                        enabled,
                        generation,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::SetGrants => {
                keys(&input, &["repository_id", "grants", "operation_id"])?;
                let repository_id = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repository_id)?, true, true)?;
                let grants: Grants = serde_json::from_value(
                    input
                        .get("grants")
                        .cloned()
                        .ok_or_else(|| ApiError::invalid("grants required"))?,
                )
                .map_err(ApiError::invalid)?;
                encode(
                    engine
                        .set_grants(
                            repository_id,
                            grants,
                            &identity.principal,
                            operation(&input)?,
                        )
                        .map_err(error)?,
                )
            }
            crate::BackendAction::Resolve => {
                keys(&input, &["branch_id", "path", "commit_id"])?;
                let branch_id = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch_id)?, false, false)?;
                let branch = engine.state().branches.get(branch_id).unwrap();
                let path = field(&input, "path")?;
                name_policy::validate_path(path).map_err(ApiError::invalid)?;
                let commit_id = input
                    .get("commit_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&branch.head_commit_id);
                let commit = engine
                    .state()
                    .commits
                    .get(commit_id)
                    .filter(|commit| commit.history_tree_id == branch.history_tree_id)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "commit not found"))?;
                let key = name_policy::path_key(path);
                let entries = engine.list_tree_at(branch_id, commit_id).map_err(error)?;
                let entry = entries
                    .iter()
                    .find(|entry| entry.lookup_key == key)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "entry not found"))?;
                let mut response = json!({"branch_id":branch_id,"commit_id":commit_id,"entry":entry,"version":engine.state().versions.get(&entry.version_id)});
                if entry.kind == "file" {
                    response.as_object_mut().unwrap().extend(
                        content_wire(
                            &engine,
                            Some(&entry.version_id),
                            pending_entry_source(&engine, branch_id, commit, entry),
                        )
                        .as_object()
                        .unwrap()
                        .clone(),
                    );
                }
                Ok(response)
            }
            crate::BackendAction::Workspace => sanitized(workspace_authorize(
                &engine,
                identity,
                field(&input, "workspace_id")?,
            )?),
            crate::BackendAction::Plan => {
                let plan = engine.plan(field(&input, "plan_id")?).map_err(error)?;
                if plan.principal != identity.principal && !identity.admin {
                    return Err(ApiError::forbidden());
                }
                authorize(
                    identity,
                    repo_for_history(&engine, field(&plan.input, "history_tree_id")?)?,
                    true,
                    plan.kind != "historical_save",
                )?;
                encode(plan)
            }
            crate::BackendAction::CreateRepository => {
                identity.require_admin()?;
                keys(
                    &input,
                    &["name", "primary_slot_id", "target_copies", "operation_id"],
                )?;
                let replicas = input
                    .get("target_copies")
                    .and_then(Value::as_u64)
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or_else(|| ApiError::invalid("target_copies must be a positive integer"))?;
                encode(
                    engine
                        .create_repository(
                            field(&input, "name")?,
                            field(&input, "primary_slot_id")?,
                            replicas,
                            &identity.principal,
                            operation(&input)?,
                        )
                        .map_err(error)?,
                )
            }
            crate::BackendAction::WriteSession => {
                keys(&input, &["branch_id"])?;
                let branch = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch)?, true, false)?;
                let workspace = engine
                    .create_workspace(branch, &identity.principal)
                    .map_err(error)?;
                Ok(
                    json!({"write_session_id":workspace.workspace_id,"workspace_id":workspace.workspace_id,"branch_id":workspace.branch_id,"principal":workspace.principal,"fencing_token":workspace.fencing_token.to_string(),"expires_at":workspace.created_at.saturating_add(engine.config().workspace_ttl_seconds).to_string(),"scope":"workspace","state":workspace.state}),
                )
            }
            crate::BackendAction::Compare => {
                keys(&input, &["left_commit_id", "right_commit_id"])?;
                let left = field(&input, "left_commit_id")?;
                let right = field(&input, "right_commit_id")?;
                for id in [left, right] {
                    let commit = engine.state().commits.get(id).ok_or_else(|| {
                        ApiError::new(404, "NOT_FOUND", "comparison commit not found")
                    })?;
                    authorize(
                        identity,
                        repo_for_history(&engine, &commit.history_tree_id)?,
                        false,
                        false,
                    )?;
                }
                engine.compare_commits(left, right).map_err(error)
            }
            crate::BackendAction::PlanCancel => {
                keys(&input, &["plan_id", "operation_id"])?;
                let plan = engine.plan(field(&input, "plan_id")?).map_err(error)?;
                if plan.principal != identity.principal && !identity.admin {
                    return Err(ApiError::forbidden());
                }
                authorize(
                    identity,
                    repo_for_history(&engine, field(&plan.input, "history_tree_id")?)?,
                    true,
                    plan.kind != "historical_save",
                )?;
                encode(
                    engine
                        .cancel_plan(
                            field(&input, "plan_id")?,
                            &identity.principal,
                            operation(&input)?,
                        )
                        .map_err(error)?,
                )
            }
            crate::BackendAction::CreateWorkspace => {
                keys(&input, &["branch_id", "source_commit_id"])?;
                let branch = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch)?, true, false)?;
                let workspace =
                    if let Some(source) = input.get("source_commit_id").and_then(Value::as_str) {
                        engine
                            .create_historical_workspace(branch, source, &identity.principal)
                            .map_err(error)?
                    } else {
                        engine
                            .create_workspace(branch, &identity.principal)
                            .map_err(error)?
                    };
                sanitized(workspace)
            }
            crate::BackendAction::Branch => {
                let id = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, id)?, false, false)?;
                branch_wire(
                    &engine,
                    engine
                        .state()
                        .branches
                        .get(id)
                        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "branch not found"))?,
                )
            }
            crate::BackendAction::HistoricalSavePlan => {
                keys(
                    &input,
                    &[
                        "history_tree_id",
                        "source_commit_id",
                        "name",
                        "workspace_id",
                        "primary_slot_id",
                        "target_copies",
                    ],
                )?;
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, true, false)?;
                let mut options = input.clone();
                options.as_object_mut().unwrap().remove("history_tree_id");
                let options: storage_engine::HistoricalSaveOptions =
                    serde_json::from_value(options).map_err(ApiError::invalid)?;
                encode(
                    engine
                        .create_historical_save_plan(history, options, &identity.principal)
                        .map_err(error)?,
                )
            }
            crate::BackendAction::WorkspaceAuthorize => {
                workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                Ok(json!({"authorized":true}))
            }
            crate::BackendAction::WorkspaceSelectionAuthorize => {
                keys(&input, &["workspace_id", "path"])?;
                let id = field(&input, "workspace_id")?;
                workspace_authorize(&engine, identity, id)?;
                engine
                    .validate_workspace_file_selection(id, field(&input, "path")?)
                    .map_err(error)?;
                Ok(json!({"authorized":true}))
            }
            crate::BackendAction::UploadAuthorize => {
                keys(&input, &["workspace_id", "item_id", "logical_size"])?;
                let workspace =
                    workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                storage_contract::validate_id(field(&input, "item_id")?, "entry")
                    .map_err(ApiError::invalid)?;
                if workspace.state != "open" {
                    return Err(ApiError::conflict(
                        "WORKSPACE_CLOSED",
                        "workspace is no longer open for upload",
                    ));
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(ApiError::invalid)?
                    .as_secs();
                if now.saturating_sub(workspace.created_at) > engine.config().workspace_ttl_seconds
                {
                    return Err(ApiError::conflict(
                        "LEASE_EXPIRED",
                        "workspace lease expired; retained data remains available",
                    ));
                }
                if input.get("logical_size").is_some() {
                    let size = storage_contract::parse_u64_decimal(field(&input, "logical_size")?)
                        .map_err(ApiError::invalid)?;
                    if size > engine.config().max_file_bytes
                        || size > engine.config().max_workspace_bytes
                    {
                        return Err(ApiError::new(
                            413,
                            "RESOURCE_LIMIT_EXCEEDED",
                            "declared upload exceeds the pool file or workspace limit",
                        ));
                    }
                }
                Ok(json!({"authorized":true}))
            }
            crate::BackendAction::UploadStatus => {
                let workspace =
                    workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                let item = field(&input, "item_id")?;
                let staged = workspace
                    .files
                    .get(item)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "upload item not found"))?;
                let sealed = matches!(workspace.state.as_str(), "sealed" | "committed");
                let manifest = if sealed {
                    Some(storage_engine::workspace_manifest_hash(&workspace).map_err(error)?)
                } else {
                    None
                };
                Ok(
                    json!({"workspace_id":workspace.workspace_id,"item_id":item,"upload_generation":staged.dirty_generation.to_string(),"logical_size":staged.size.to_string(),"content_hash":staged.content_hash,"state":if sealed{"SEALED"}else{"COMPLETE"},"durable_ranges":if staged.size>0{vec![json!({"offset":"0","length":staged.size.to_string(),"content_hash":staged.content_hash})]}else{vec![]},"sealed_manifest_hash":manifest}),
                )
            }
            crate::BackendAction::Namespace => {
                keys(&input, &["workspace_id", "kind", "path", "destination"])?;
                let workspace = field(&input, "workspace_id")?;
                let change = NamespaceOperation {
                    kind: field(&input, "kind")?.into(),
                    path: field(&input, "path")?.into(),
                    destination: input
                        .get("destination")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                };
                sanitized(Application::new(&mut engine, identity).stage_namespace(
                    application::StageNamespace {
                        workspace_id: workspace.into(),
                        change,
                    },
                )?)
            }
            crate::BackendAction::SealWorkspace => {
                keys(&input, &["workspace_id"])?;
                let workspace =
                    workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                engine
                    .seal_workspace(&workspace.workspace_id, &workspace.principal)
                    .map_err(error)
            }
            crate::BackendAction::CommitWorkspace
            | crate::BackendAction::CommitWorkspaceInternal => {
                if action == crate::BackendAction::CommitWorkspace {
                    keys(
                        &input,
                        &[
                            "workspace_id",
                            "operation_id",
                            "repository_id",
                            "branch_id",
                            "expected_head",
                            "fencing_token",
                            "message",
                            "requested_protection",
                            "file_placement",
                            "expected_topology_generation",
                            "sealed_manifest_hash",
                            "request_digest",
                        ],
                    )?;
                } else {
                    keys(
                        &input,
                        &[
                            "workspace_id",
                            "operation_id",
                            "expected_head",
                            "fencing_token",
                            "message",
                            "request_digest",
                        ],
                    )?;
                }
                // Keep strict string presence checks at the compatibility wire edge.
                for key in [
                    "workspace_id",
                    "operation_id",
                    "expected_head",
                    "fencing_token",
                ] {
                    field(&input, key)?;
                }
                let workspace_id = field(&input, "workspace_id")?.to_owned();
                let plan = if action == crate::BackendAction::CommitWorkspace {
                    let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                    Application::new(&mut engine, identity)
                        .prepare_commit_workspace_public(command)?
                } else {
                    let command = application::CommitWorkspace {
                        workspace_id: field(&input, "workspace_id")?.into(),
                        operation_id: operation(&input)?.into(),
                        expected_head: field(&input, "expected_head")?.into(),
                        fencing_token: storage_contract::parse_u64_decimal(field(
                            &input,
                            "fencing_token",
                        )?)
                        .map_err(ApiError::invalid)?,
                        message: input
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        request_digest: input
                            .get("request_digest")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    };
                    Application::new(&mut engine, identity).prepare_commit_workspace(command)?
                };
                drop(engine);
                #[cfg(test)]
                if let Some(hook) = self.commit_preparation_hook.lock().unwrap().clone() {
                    hook();
                }
                let prepared = plan.execute().map_err(error)?;
                let mut engine = self.engine.lock().map_err(|_| {
                    ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart")
                })?;
                let current = current_protected_identity(&engine, identity)?;
                let result = Application::new(&mut engine, &current)
                    .finish_workspace_commit(&workspace_id, prepared)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::AbortWorkspace => {
                keys(&input, &["workspace_id"])?;
                let id = field(&input, "workspace_id")?;
                workspace_authorize(&engine, identity, id)?;
                engine
                    .abort_workspace(id, &identity.principal)
                    .map_err(error)?;
                Ok(json!({"workspace_id":id,"state":"aborted"}))
            }
            crate::BackendAction::CreateBranch => {
                for key in ["repository_id", "source_commit_id", "name", "operation_id"] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                let branch = Application::new(&mut engine, identity).create_branch(command)?;
                branch_wire(&engine, &branch)
            }
            crate::BackendAction::Archive => {
                for key in ["branch_id", "operation_id"] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                encode(Application::new(&mut engine, identity).archive_branch(command)?)
            }
            crate::BackendAction::MergeLink => {
                for key in [
                    "history_tree_id",
                    "source_branch_id",
                    "source_commit_id",
                    "target_branch_id",
                    "target_commit_id",
                    "operation_id",
                ] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                encode(Application::new(&mut engine, identity).merge_link(command)?)
            }
            crate::BackendAction::History => {
                let file = field(&input, "file_id")?;
                let versions = engine.file_history(file).map_err(error)?;
                let visible = versions
                    .into_iter()
                    .filter(|version| {
                        repo_for_history(&engine, &version.history_tree_id)
                            .is_ok_and(|repo| authorize(identity, repo, false, false).is_ok())
                    })
                    .collect::<Vec<_>>();
                if visible.is_empty() {
                    return Err(ApiError::new(404, "NOT_FOUND", "file history not found"));
                }
                Ok(json!({"file_id":file,"versions":visible}))
            }
            crate::BackendAction::Graph => {
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, false, false)?;
                encode(engine.graph(history).map_err(error)?)
            }
            crate::BackendAction::Commit => {
                let commit = engine
                    .state()
                    .commits
                    .get(field(&input, "commit_id")?)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "commit not found"))?;
                authorize(
                    identity,
                    repo_for_history(&engine, &commit.history_tree_id)?,
                    false,
                    false,
                )?;
                encode(commit)
            }
            crate::BackendAction::Version | crate::BackendAction::Placement => {
                let id = field(&input, "version_id")?;
                let version = engine
                    .state()
                    .versions
                    .get(id)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "version not found"))?;
                authorize(
                    identity,
                    repo_for_history(&engine, &version.history_tree_id)?,
                    false,
                    false,
                )?;
                if action == crate::BackendAction::Version {
                    let mut value = encode(version)?;
                    value["content_state"] = engine
                        .state()
                        .content_states
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| "available".into())
                        .into();
                    Ok(value)
                } else {
                    encode(
                        engine.state().placements.get(id).ok_or_else(|| {
                            ApiError::new(404, "NOT_FOUND", "placement not found")
                        })?,
                    )
                }
            }
            crate::BackendAction::Operation => {
                let operation_id = field(&input, "operation_id")?;
                let operation = engine.operation(operation_id).map_err(error)?;
                if !identity.admin
                    && engine.operation_owner(operation_id).as_deref() != Some(&identity.principal)
                {
                    return Err(ApiError::forbidden());
                }
                if let Some(history) = &operation.history_tree_id {
                    authorize(identity, repo_for_history(&engine, history)?, false, false)?;
                } else {
                    identity.require_admin()?;
                }
                operation_wire(&engine, operation)
            }
            crate::BackendAction::PurgePlan => {
                keys(&input, &["history_tree_id", "version_ids"])?;
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, true, true)?;
                let versions: Vec<String> = serde_json::from_value(
                    input
                        .get("version_ids")
                        .cloned()
                        .ok_or_else(|| ApiError::invalid("version_ids required"))?,
                )
                .map_err(ApiError::invalid)?;
                encode(
                    engine
                        .create_purge_plan(history, versions, &identity.principal)
                        .map_err(error)?,
                )
            }
            crate::BackendAction::SplitPlan => {
                keys(&input, &["history_tree_id", "cut_commit_id"])?;
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, true, true)?;
                encode(
                    engine
                        .create_split_plan(
                            history,
                            field(&input, "cut_commit_id")?,
                            &identity.principal,
                        )
                        .map_err(error)?,
                )
            }
            crate::BackendAction::PlanCommit => {
                for key in ["plan_id", "operation_id", "plan_digest"] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                encode(Application::new(&mut engine, identity).commit_plan(command)?)
            }
            crate::BackendAction::Policy => {
                let policy = engine
                    .state()
                    .protection_policies
                    .get(field(&input, "policy_id")?)
                    .ok_or_else(|| {
                        ApiError::new(404, "NOT_FOUND", "protection policy not found")
                    })?;
                authorize(
                    identity,
                    repository(&engine, &policy.repository_id)?,
                    false,
                    false,
                )?;
                encode(policy)
            }
            crate::BackendAction::PlacementRule => {
                let rule = engine
                    .state()
                    .placement_rules
                    .get(field(&input, "rule_id")?)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "placement rule not found"))?;
                authorize(
                    identity,
                    repository(&engine, &rule.repository_id)?,
                    false,
                    false,
                )?;
                encode(rule)
            }
            crate::BackendAction::UpdatePolicy => {
                keys(
                    &input,
                    &[
                        "policy_id",
                        "repository_id",
                        "path_prefix",
                        "target_copies",
                        "expected_generation",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, true)?;
                // Retain the transport's key/admission error ordering while the
                // application also enforces ACLs for non-HTTP callers.
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                let result =
                    Application::new(&mut engine, identity).update_protection_policy(command)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::UpdatePlacementRule => {
                keys(
                    &input,
                    &[
                        "rule_id",
                        "repository_id",
                        "path_prefix",
                        "primary_slot_id",
                        "expected_generation",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, true)?;
                // Retain the transport's key/admission error ordering while the
                // application also enforces ACLs for non-HTTP callers.
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                let result =
                    Application::new(&mut engine, identity).update_placement_rule(command)?;
                operation_wire(&engine, result)
            }
            crate::BackendAction::Check => {
                identity.require_admin()?;
                encode(engine.check().map_err(error)?)
            }
            crate::BackendAction::Disks => {
                identity.require_admin()?;
                let observation = engine.prepare_disk_observation();
                drop(engine);
                encode(json!({"disks":observation.status().map_err(error)?}))
            }
            crate::BackendAction::Topology => {
                identity.require_admin()?;
                Ok(
                    json!({"topology_id":"default","generation":engine.state().topology_generation.to_string(),"slots":engine.config().disks.iter().map(|disk|json!({"slot_id":disk.slot_id,"disk_id":disk.disk_id,"level":disk.level,"upstream_slot_id":disk.upstream_slot_id,"metadata_member":disk.metadata_member,"binding_generation":disk.binding_generation.to_string()})).collect::<Vec<_>>()}),
                )
            }
            crate::BackendAction::Events => {
                let after = storage_contract::parse_u64_decimal(
                    input
                        .get("query")
                        .and_then(|query| query.get("after"))
                        .and_then(Value::as_str)
                        .unwrap_or("0"),
                )
                .map_err(ApiError::invalid)?;
                if after > engine.state().sequence {
                    return Err(ApiError::new(
                        400,
                        "INVALID_CURSOR",
                        "event cursor is ahead of the published sequence",
                    ));
                }
                let mut events = Vec::new();
                let mut inspected = after;
                for event in engine
                    .state()
                    .events
                    .iter()
                    .filter(|event| event.sequence > after)
                {
                    inspected = event.sequence;
                    if identity.admin
                        || engine
                            .state()
                            .operations
                            .get(&event.operation_id)
                            .and_then(|op| op.history_tree_id.as_ref())
                            .and_then(|history| repo_for_history(&engine, history).ok())
                            .is_some_and(|repo| authorize(identity, repo, false, false).is_ok())
                    {
                        events.push(event);
                    }
                    if events.len() >= 1000 {
                        break;
                    }
                }
                Ok(json!({"events":events,"next_cursor":inspected.to_string()}))
            }
            crate::BackendAction::RegisterDisk => {
                identity.require_admin()?;
                Err(ApiError::new(
                    409,
                    "OFFLINE_CONFIGURATION_REQUIRED",
                    "disk registration requires an explicit offline configuration and topology migration",
                ))
            }
        }
    }
    fn upload(
        &self,
        identity: &Identity,
        workspace: &str,
        _item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, ApiError> {
        let engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        let current = workspace_authorize(&engine, identity, workspace)?;
        let generation = current
            .files
            .get(_item)
            .map(|file| file.dirty_generation.checked_add(1))
            .unwrap_or(Some(1))
            .ok_or_else(|| {
                error(storage_engine::Error::new(
                    "RESOURCE_LIMIT",
                    "upload generation exhausted",
                ))
            })?;
        let plan = engine
            .prepare_upload(workspace, _item, path, source, generation)
            .map_err(error)?;
        drop(engine);
        let prepared = plan.execute().map_err(error)?;
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        finish_authorized_upload(&mut engine, identity, workspace, prepared)
    }
    fn upload_generation(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
        generation: u64,
    ) -> Result<Value, ApiError> {
        let engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        workspace_authorize(&engine, identity, workspace)?;
        let plan = engine
            .prepare_upload(workspace, item, path, source, generation)
            .map_err(error)?;
        drop(engine);
        let prepared = plan.execute().map_err(error)?;
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        // Admission and current ACLs may have changed while bytes were copied.
        finish_authorized_upload(&mut engine, identity, workspace, prepared)
    }
    fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.range_content(identity, version)?.fully_verified()
    }
    fn range_content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.open_version_content(identity, version)
    }
    fn workspace_content(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
    ) -> Result<Content, ApiError> {
        self.open_workspace_content(identity, workspace, item)
    }
    fn checksums(&self, identity: &Identity, version: &str) -> Result<Vec<u8>, ApiError> {
        self.read_version_checksums(identity, version)
    }
}

#[cfg(test)]
mod authorization_tests;
